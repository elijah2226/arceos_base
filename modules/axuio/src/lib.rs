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

use alloc::string::ToString;
use alloc::vec;
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
