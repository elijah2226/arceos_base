// /starry/.arceos/modules/axuio/src/lib.rs

#![no_std]

#[macro_use]
extern crate axlog;
extern crate alloc;
extern crate axerrno; // 【【【新增】】】

mod device;
pub mod file;
mod manager;

pub use device::UioMemoryRegion;
pub use manager::{register_device, uio_irq_dispatcher};

use alloc::{string::ToString, vec};
use axdevice_event;
use axerrno::{AxError, AxResult};

pub fn init() {
    info!("axuio module initialized.");
}

pub fn create_device_file(device_id: usize) -> AxResult {
    use alloc::sync::Arc;
    use file::UioDeviceFile;

    if let Some(devfs_instance) = axfs::DEVFS::get() {
        let uio_node = Arc::new(UioDeviceFile::new(device_id)?);

        let device_name = match device_id {
            0 => "uio0",
            1 => "uio1",
            _ => return axerrno::ax_err!(NoMemory, "device id too large"),
        };

        devfs_instance.add(device_name, uio_node);

        info!("Successfully registered UIO device at /dev/{}", device_name);
        Ok(())
    } else {
        axerrno::ax_err!(NotFound, "DEVFS is not initialized or feature not enabled")
    }
}

/// 注册一个虚拟的 UIO 设备用于测试。
///
/// 在真实的系统中，这个调用会来自具体的设备驱动程序，
/// 比如 virtio-net 驱动在初始化时会调用 `register_device`。
pub fn test_register_dummy_device() {
    info!("Attempting to register a dummy UIO device for testing...");
    // 模拟一个设备，它有 64KB 的内存区域和 virtio-pci 的中断号 11
    // (在 QEMU aarch64 上，virtio-net 的中断号通常是 33，在 x86_64 上是 11)
    let paddr = axhal::mem::PhysAddr::from(0x300000); // 随便选一个未使用的物理地址
    let size = 64 * 1024; // 64KB
    let irq = 11; // virtio-pci IRQ on x86_64 QEMU

    match register_device(
        "dummy-virtio-net".to_string(),
        "0.1.0".to_string(),
        vec![device::UioMemoryRegion { paddr, size }],
        Some(irq),
    ) {
        Ok(id) => info!("Dummy UIO device registered with ID: {}", id),
        Err(e) => error!("Failed to register dummy UIO device: {:?}", e),
    }
}

/// Registers all devices discovered by axdriver as UIO devices.
/// This should be called after axdriver has finished probing devices.
pub fn register_discovered_devices() {
    info!("axuio: Registering discovered devices from axdevice_event...");

    let mut devices_to_register = vec::Vec::new();
    // 获取锁，并将已发现的设备信息移动到本地 vec 中，以便在释放锁后处理
    {
        let mut discovered_devices_lock = axdevice_event::DISCOVERED_DEVICES.lock();
        // 使用 drain() 迭代器，将所有元素移动到新的 Vec 中并清空原始 Vec
        devices_to_register.extend(discovered_devices_lock.drain(..));
    }

    for (i, info) in devices_to_register.into_iter().enumerate() {
        if let Some((paddr, size)) = info.mmio_region {
            // 只有当设备有 MMIO 区域时才注册为 UIO 设备
            info!(
                "axuio: Registering UIO for device {}: {} ({}) at PA:{:#x}, size={:#x}, irq={:?}",
                i, info.name, info.pci_bdf, paddr, size, info.irq_num
            );
            match manager::register_device(
                info.name,
                info.pci_bdf, // 使用 PCI BDF 作为版本或类似唯一字符串
                vec![device::UioMemoryRegion { paddr, size }],
                info.irq_num, // IRQ 可以是 None
            ) {
                Ok(id) => info!("axuio: Device registered as /dev/uio{}", id),
                Err(e) => error!("axuio: Failed to register device as UIO: {:?}", e),
            }
        } else {
            // 如果设备没有 MMIO 区域，则不注册 UIO 设备
            warn!(
                "axuio: Discovered device {} has no MMIO region, skipping UIO registration.",
                info.name
            );
        }
    }
    info!("axuio: Finished registering discovered devices.");
}

/// 注册 HPET (High Precision Event Timer) 作为一个 UIO 设备。
///
/// HPET 是一个理想的 UIO Demo，因为它拥有一个标准化的、非 PCI 发现的 MMIO 区域，
/// 其物理地址是固定的。这使得我们可以直接注册它，而无需复杂的总线扫描。
pub fn register_hpet_device() -> AxResult<()> {
    info!("Attempting to register HPET as a UIO device...");

    // HPET 规范中定义的标准物理基地址
    let hpet_paddr = axhal::mem::PhysAddr::from(0xFED00000);
    // HPET 寄存器块的大小是 1KB
    let hpet_size = 0x400;
    // 在 QEMU 中，HPET 的中断通常是 IRQ 8。我们注册它来测试中断处理路径。
    let hpet_irq = Some(8);

    match manager::register_device(
        "hpet-0".to_string(), // 设备名称
        "1.0.0".to_string(),  // 版本字符串
        vec![UioMemoryRegion {
            paddr: hpet_paddr,
            size: hpet_size,
        }],
        hpet_irq,
    ) {
        Ok(id) => {
            info!("HPET successfully registered as UIO device with ID: {}", id);
            Ok(()) // 返回成功
        }
        Err(e) => {
            error!("Failed to register HPET UIO device: {:?}", e);
            Err(e) // 将错误传递出去
        }
    }
}

/// 注册传统的 VGA 文本模式缓冲区为一个 UIO 设备。
///
/// 这是一个绝佳的、纯 MMIO 写入操作的 Demo，因为任何写入到
/// 这块内存区域的数据都会立刻显示在屏幕上，提供即时视觉反馈。
pub fn register_vga_text_device() -> AxResult<()> {
    info!("尝试注册 VGA 文本缓冲区为 UIO 设备...");

    // VGA 文本模式缓冲区的经典物理地址
    let vga_paddr = axhal::mem::PhysAddr::from(0xB8000);
    // 显存缓冲区是 80x25 个字符，每个字符占 2 字节（字符编码 + 颜色属性）。
    // 总大小为 80 * 25 * 2 = 4000 字节 (0xFA0)。我们映射完整的 4KB (0x1000) 更方便。
    let vga_size = 0x1000;
    // VGA 文本缓冲区本身不会产生中断。
    let vga_irq = None;

    match manager::register_device(
        "vga-text-0".to_string(), // 设备名
        "1.0.0".to_string(),      // 版本
        vec![UioMemoryRegion {
            paddr: vga_paddr,
            size: vga_size,
        }],
        vga_irq, // 此设备没有中断
    ) {
        Ok(id) => {
            info!("VGA 文本缓冲区成功注册为 UIO 设备，ID 为: {}", id);
            Ok(())
        }
        Err(e) => {
            error!("注册 VGA 文本缓冲区 UIO 设备失败: {:?}", e);
            Err(e)
        }
    }
}
