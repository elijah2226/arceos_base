// /starry/.arceos/modules/axuio/src/file.rs

use super::device::UioDevice;
use super::manager;
use alloc::sync::Arc;
use axerrno::{AxError, AxResult}; // 新的错误处理
use axfs_vfs::{VfsNodeAttr, VfsNodeOps, VfsNodePerm, VfsNodeType};
use axhal::mem::{PhysAddr, VirtAddr}; // mmap 仍然需要
use axio::{Read, Result, Seek, SeekFrom, Write};

pub struct UioDeviceFile {
    device: Arc<UioDevice>,
}

impl UioDeviceFile {
    pub fn new(device_id: usize) -> AxResult<Self> {
        let device = manager::get_device(device_id).ok_or(AxError::NotFound)?; // 使用 AxError
        Ok(Self { device })
    }
    // --- 【【【新增：这是 mmap 的正确实现方式】】】 ---
    /// 处理针对此 UIO 设备的内存映射请求。
    ///
    /// # 关于 VFS 集成的说明
    ///
    /// `axfs-vfs` 的 `VfsNodeOps` trait 没有 `mmap` 方法。
    /// 一个真正的 `mmap` 系统调用处理程序需要：
    /// 1. 根据文件描述符获取 `VfsNodeRef`。
    /// 2. 使用 `node.as_any().downcast_ref::<UioDeviceFile>()` 来获取一个
    ///    指向此结构体的具体引用。
    /// 3. 如果向下转型成功，调用此 `handle_mmap` 方法来获取用于映射的物理地址。
    ///
    /// 此函数返回需要被映射的物理地址。VFS/MMU 子系统负责实际的页表操作。
    pub fn handle_mmap(&self, offset: usize, length: usize) -> AxResult<axhal::mem::PhysAddr> {
        // UIO 规范: offset 对应于第 N 个内存区域。
        // offset = 0 -> mem[0], offset = PAGE_SIZE -> mem[1], 等等。
        // --- 【【【修改：使用 axhal::mem::PAGE_SIZE】】】 ---
        let page_size = axhal::mem::PAGE_SIZE_4K;
        if offset % page_size != 0 {
            return axerrno::ax_err!(InvalidInput, "mmap offset must be a multiple of PAGE_SIZE");
        }
        let mem_region_index = offset / page_size;

        if let Some(mem_region) = self.device.mem_regions.get(mem_region_index) {
            if length > mem_region.size {
                return axerrno::ax_err!(
                    InvalidInput,
                    "mmap length exceeds the memory region size"
                );
            }
            info!(
                "UIO mmap: providing paddr {:#x} for region {}",
                mem_region.paddr, mem_region_index
            );
            Ok(mem_region.paddr)
        } else {
            axerrno::ax_err!(NotFound, "Invalid UIO memory region index")
        }
    }
}

impl VfsNodeOps for UioDeviceFile {
    // --- get_attr: 使用正确的构造函数 ---
    fn get_attr(&self) -> AxResult<VfsNodeAttr> {
        Ok(VfsNodeAttr::new(
            // 使用 from_bits_truncate 从整数创建权限
            VfsNodePerm::from_bits_truncate(0o666),
            // 类型是文件
            VfsNodeType::File,
            // 大小和块数都为 0
            0,
            0,
        ))
    }

    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> AxResult<usize> {
        if let Some(irq) = &self.device.irq {
            // 1. 检查计数器。如果计数器大于0，说明已经有中断发生了。
            //    我们消费一个中断，然后立即返回。
            let mut count_lock = irq.count.lock();
            if *count_lock > 0 {
                *count_lock -= 1;
                // 必须在返回前释放锁！
                drop(count_lock);

                // UIO spec 要求返回中断计数值的大小，这里简化为返回4个字节
                let one: u32 = 1;
                let n = buf.len().min(4);
                buf[..n].copy_from_slice(&one.to_ne_bytes()[..n]);
                return Ok(n);
            }
            // 如果计数器为0，释放锁并准备等待
            drop(count_lock);

            // 2. 等待中断发生。
            //    wait() 会阻塞当前任务，直到被 uio_irq_dispatcher 唤醒。
            irq.wait_queue.wait();

            // 3. 被唤醒后，我们知道一个中断刚刚发生并被处理了。
            //    我们消费这个中断。
            *irq.count.lock() -= 1;

            // 4. 返回数据给用户。
            let one: u32 = 1;
            let n = buf.len().min(4);
            buf[..n].copy_from_slice(&one.to_ne_bytes()[..n]);
            Ok(n)
        } else {
            Err(AxError::Unsupported)
        }
    }

    // --- write_at: 只修复错误类型名 ---
    fn write_at(&self, _offset: u64, _buf: &[u8]) -> AxResult<usize> {
        Err(AxError::Unsupported)
    }
    // --- 其他方法将使用 VfsNodeOps trait 中的默认实现 ---
    // --- 它们默认返回 ax_err!(Unsupported) 或类似错误，这对于文件节点是正确的 ---
    // --- 【【【新增：必须实现 as_any 以支持向下转型】】】 ---
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}
