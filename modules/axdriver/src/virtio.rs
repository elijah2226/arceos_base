use crate::alloc::string::ToString;
use axalloc::global_allocator;
use axdevice_event;
use axdriver_base::{BaseDriverOps, DevResult, DeviceType};
use axdriver_pci::BarInfo;
use axdriver_virtio::{BufferDirection, PhysAddr, VirtIoHal};
use axhal::mem::PhysAddr as PhysAddrTrait;
use axhal::mem::{phys_to_virt, virt_to_phys};
use cfg_if::cfg_if;
use core::{marker::PhantomData, ptr::NonNull};

use crate::{AxDeviceEnum, drivers::DriverProbe};

cfg_if! {
    if #[cfg(bus = "pci")] {
        use axdriver_pci::{PciRoot, DeviceFunction, DeviceFunctionInfo};
        type VirtIoTransport = axdriver_virtio::PciTransport;
    } else if #[cfg(bus =  "mmio")] {
        type VirtIoTransport = axdriver_virtio::MmioTransport;
    }
}

/// A trait for VirtIO device meta information.
pub trait VirtIoDevMeta {
    const DEVICE_TYPE: DeviceType;

    type Device: BaseDriverOps;
    type Driver = VirtIoDriver<Self>;

    fn try_new(transport: VirtIoTransport) -> DevResult<AxDeviceEnum>;
}

cfg_if! {
    if #[cfg(net_dev = "virtio-net")] {
        pub struct VirtIoNet;

        impl VirtIoDevMeta for VirtIoNet {
            const DEVICE_TYPE: DeviceType = DeviceType::Net;
            type Device = axdriver_virtio::VirtIoNetDev<VirtIoHalImpl, VirtIoTransport, 64>;

            fn try_new(transport: VirtIoTransport) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_net(Self::Device::try_new(transport)?))
            }
        }
    }
}

cfg_if! {
    if #[cfg(block_dev = "virtio-blk")] {
        pub struct VirtIoBlk;

        impl VirtIoDevMeta for VirtIoBlk {
            const DEVICE_TYPE: DeviceType = DeviceType::Block;
            type Device = axdriver_virtio::VirtIoBlkDev<VirtIoHalImpl, VirtIoTransport>;

            fn try_new(transport: VirtIoTransport) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_block(Self::Device::try_new(transport)?))
            }
        }
    }
}

cfg_if! {
    if #[cfg(display_dev = "virtio-gpu")] {
        pub struct VirtIoGpu;

        impl VirtIoDevMeta for VirtIoGpu {
            const DEVICE_TYPE: DeviceType = DeviceType::Display;
            type Device = axdriver_virtio::VirtIoGpuDev<VirtIoHalImpl, VirtIoTransport>;

            fn try_new(transport: VirtIoTransport) -> DevResult<AxDeviceEnum> {
                Ok(AxDeviceEnum::from_display(Self::Device::try_new(transport)?))
            }
        }
    }
}

/// A common driver for all VirtIO devices that implements [`DriverProbe`].
pub struct VirtIoDriver<D: VirtIoDevMeta + ?Sized>(PhantomData<D>);

impl<D: VirtIoDevMeta> DriverProbe for VirtIoDriver<D> {
    #[cfg(bus = "mmio")]
    fn probe_mmio(mmio_base: usize, mmio_size: usize) -> Option<AxDeviceEnum> {
        let base_vaddr = phys_to_virt(mmio_base.into());
        if let Some((ty, transport)) =
            axdriver_virtio::probe_mmio_device(base_vaddr.as_mut_ptr(), mmio_size)
            && ty == D::DEVICE_TYPE
        {
            match D::try_new(transport) {
                Ok(dev) => return Some(dev),
                Err(e) => {
                    warn!(
                        "failed to initialize MMIO device at [PA:{:#x}, PA:{:#x}): {:?}",
                        mmio_base,
                        mmio_base + mmio_size,
                        e
                    );
                    return None;
                }
            }
        }
        None
    }

    #[cfg(bus = "pci")]
    fn probe_pci(
        root: &mut PciRoot,
        bdf: DeviceFunction,
        dev_info: &DeviceFunctionInfo,
    ) -> Option<AxDeviceEnum> {
        if dev_info.vendor_id != 0x1af4 {
            return None;
        }
        match (D::DEVICE_TYPE, dev_info.device_id) {
            (DeviceType::Net, 0x1000) | (DeviceType::Net, 0x1041) => {}
            (DeviceType::Block, 0x1001) | (DeviceType::Block, 0x1042) => {}
            (DeviceType::Display, 0x1050) => {}
            _ => return None,
        }

        info!(
            "PCI Probe Debug: Device {}: VendorID={:#x}, DeviceID={:#x}",
            bdf, dev_info.vendor_id, dev_info.device_id
        );

        let bar0_info: Result<BarInfo, axdriver_pci::PciError> = root.bar_info(bdf, 0);
        // 打印 BAR0 的原始信息
        info!(
            "PCI Probe Debug: Device {}: BAR0 raw result: {:?}",
            bdf, bar0_info
        );

        let (pci_bar_paddr_raw, pci_bar_size, is_memory_bar) = match bar0_info {
            Ok(BarInfo::Memory { address, size, .. }) => {
                info!(
                    "Virtio PCI device {}: BAR0 is Memory BAR (addr={:#x}, size={:#x}).",
                    bdf, address, size
                );
                (address as usize, size as usize, true)
            }
            Ok(BarInfo::IO { address, size }) => {
                warn!(
                    "Virtio PCI device {}: BAR0 is an IO BAR (addr={:#x}, size={:#x}). Not publishing for UIO MMIO.",
                    bdf, address, size
                );
                (0, 0, false)
            }
            Err(e) => {
                warn!(
                    "Virtio PCI device {}: Failed to get BAR0 info: {:?}",
                    bdf, e
                );
                return None;
            }
        };

        let axhal_pci_bar_paddr: PhysAddrTrait = PhysAddrTrait::from(pci_bar_paddr_raw);

        // 2. 从 PCI 配置空间读取 IRQ Line 寄存器
        let irq_byte = root.read_config_byte(bdf, 0x3C);
        info!(
            "PCI Probe Debug: Device {}: IRQ Line raw byte: {:#x}",
            bdf, irq_byte
        );
        let irq_num = if irq_byte == 0xFF || irq_byte == 0x00 {
            warn!(
                "Virtio PCI device {}: No valid IRQ line found (value: {:#x}).",
                bdf, irq_byte
            );
            0 // 记录为 0，或者你可以选择 None
        } else {
            irq_byte as usize
        };

        info!(
            "PCI Probe Debug: Device {}: Final IRQ Num for UIO: {:#x}",
            bdf, irq_num
        );

        let device_name_str = format!(
            "virtio-{}-{}",
            match D::DEVICE_TYPE {
                DeviceType::Net => "net",
                DeviceType::Block => "blk",
                DeviceType::Display => "gpu",
                _ => "Unknown",
            },
            bdf.to_string()
        );

        // 如果是 Memory BAR，就发布其地址信息；否则，mmio_region 为 None。
        let mmio_region_info: Option<(PhysAddrTrait, usize)> = if is_memory_bar {
            Some((PhysAddrTrait::from(pci_bar_paddr_raw), pci_bar_size))
        } else {
            None
        };

        info!(
            "PCI Probe Debug: Device {}: mmio_region_info content: {:?}",
            bdf, mmio_region_info
        );

        axdevice_event::publish_device_info(axdevice_event::DiscoveredDeviceInfo {
            device_type: D::DEVICE_TYPE,
            name: device_name_str,
            pci_bdf: bdf.to_string(),
            mmio_region: mmio_region_info,
            irq_num: Some(irq_num),
        });
        info!("Published device info for {}.", bdf.to_string());
        // 【【【 发布结束 】】】

        if let Some((ty, transport)) =
            axdriver_virtio::probe_pci_device::<VirtIoHalImpl>(root, bdf, dev_info)
        {
            if ty == D::DEVICE_TYPE {
                match D::try_new(transport) {
                    Ok(dev) => return Some(dev),
                    Err(e) => {
                        warn!(
                            "failed to initialize PCI device at {}({}): {:?}",
                            bdf, dev_info, e
                        );
                        return None;
                    }
                }
            }
        }
        None
    }
}

pub struct VirtIoHalImpl;

unsafe impl VirtIoHal for VirtIoHalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let vaddr = if let Ok(vaddr) = global_allocator().alloc_pages(pages, 0x1000) {
            vaddr
        } else {
            return (0, NonNull::dangling());
        };
        let paddr = virt_to_phys(vaddr.into());
        let ptr = NonNull::new(vaddr as _).unwrap();
        (paddr.as_usize(), ptr)
    }

    unsafe fn dma_dealloc(_paddr: PhysAddr, vaddr: NonNull<u8>, pages: usize) -> i32 {
        global_allocator().dealloc_pages(vaddr.as_ptr() as usize, pages);
        0
    }

    #[inline]
    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        NonNull::new(phys_to_virt(paddr.into()).as_mut_ptr()).unwrap()
    }

    #[inline]
    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        virt_to_phys(vaddr.into()).into()
    }

    #[inline]
    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}
