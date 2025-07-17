use crate::file::{File, FileLike};
use alloc::{sync::Arc, vec};
use axerrno::{LinuxError, LinuxResult};
use axhal::paging::{MappingFlags, PageSize};
use axtask::{TaskExtRef, current};
use axuio::file::UioDeviceFile;
use core::any::Any;
use linux_raw_sys::general::*;
use memory_addr::{MemoryAddr, VirtAddr, VirtAddrRange, align_up_4k};

fn downcast_file_from_axfs<T: Any>(axfs_file: &axfs::fops::File) -> Option<&T> {
    // 调用我们新加的 node() 方法，然后调用 as_any()
    axfs_file.node().as_any().downcast_ref::<T>()
}

bitflags::bitflags! {
    /// `PROT_*` flags for use with [`sys_mmap`].
    ///
    /// For `PROT_NONE`, use `ProtFlags::empty()`.
    #[derive(Debug)]
    struct MmapProt: u32 {
        /// Page can be read.
        const READ = PROT_READ;
        /// Page can be written.
        const WRITE = PROT_WRITE;
        /// Page can be executed.
        const EXEC = PROT_EXEC;
        /// Extend change to start of growsdown vma (mprotect only).
        const GROWDOWN = PROT_GROWSDOWN;
        /// Extend change to start of growsup vma (mprotect only).
        const GROWSUP = PROT_GROWSUP;
    }
}

impl From<MmapProt> for MappingFlags {
    fn from(value: MmapProt) -> Self {
        let mut flags = MappingFlags::USER;
        if value.contains(MmapProt::READ) {
            flags |= MappingFlags::READ;
        }
        if value.contains(MmapProt::WRITE) {
            flags |= MappingFlags::WRITE;
        }
        if value.contains(MmapProt::EXEC) {
            flags |= MappingFlags::EXECUTE;
        }
        flags
    }
}

bitflags::bitflags! {
    /// flags for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    #[derive(Debug)]
    struct MmapFlags: u32 {
        /// Share changes
        const SHARED = MAP_SHARED;
        /// Changes private; copy pages on write.
        const PRIVATE = MAP_PRIVATE;
        /// Map address must be exactly as requested, no matter whether it is available.
        const FIXED = MAP_FIXED;
        /// Don't use a file.
        const ANONYMOUS = MAP_ANONYMOUS;
        /// Don't check for reservations.
        const NORESERVE = MAP_NORESERVE;
        /// Allocation is for a stack.
        const STACK = MAP_STACK;
        /// Huge page
        const HUGE = MAP_HUGETLB;
        /// Huge page 1g size
        const HUGE_1GB = MAP_HUGETLB | MAP_HUGE_1GB;
    }
}

pub fn sys_mmap(
    addr: usize,
    length: usize,
    prot: u32,
    flags: u32,
    fd: i32,
    offset: isize,
) -> LinuxResult<isize> {
    let curr = current();
    let process_data = curr.task_ext().process_data();
    let mut aspace = process_data.aspace.lock();
    let permission_flags = MmapProt::from_bits_truncate(prot);
    // TODO: check illegal flags for mmap
    // An example is the flags contained none of MAP_PRIVATE, MAP_SHARED, or MAP_SHARED_VALIDATE.
    let map_flags = MmapFlags::from_bits_truncate(flags);
    if map_flags.contains(MmapFlags::PRIVATE | MmapFlags::SHARED) {
        return Err(LinuxError::EINVAL);
    }

    info!(
        "sys_mmap: addr: {:x?}, length: {:x?}, prot: {:?}, flags: {:?}, fd: {:?}, offset: {:?}",
        addr, length, permission_flags, map_flags, fd, offset
    );
    // ====================== 【【【 新增 UIO 处理逻辑 】】】 ======================
    if fd != -1 {
        // 只有文件映射才可能是 UIO
        let file_wrapper: Arc<File> = File::from_fd(fd)?;
        let axfs_file_guard = file_wrapper.inner(); // 这返回一个 MutexGuard<axfs::fops::File>
        // 尝试向下转型为 UioDeviceFile
        // 如果你的 downcast_file 放在别处，请修改路径
        if let Some(uio_file) = axfs_file_guard
            .node()
            .as_any()
            .downcast_ref::<UioDeviceFile>()
        {
            // 这就是 UIO 设备！进入特殊处理流程。
            info!("sys_mmap: Detected UIO device, using custom handler.");

            // 1. 调用 UioDeviceFile 的 handle_mmap 获取物理地址
            // 注意：C 的 mmap offset 是 isize，而我们的是 usize，需要转换
            if offset < 0 {
                return Err(LinuxError::EINVAL);
            }
            let paddr = uio_file.handle_mmap(offset as usize, length)?;

            // 2. 寻找一块可用的虚拟地址空间 (这部分逻辑可以复用你下面的代码)
            let page_size = PageSize::Size4K; // UIO 通常用 4K 页
            let start_vaddr = if map_flags.contains(MmapFlags::FIXED) {
                // ... (处理 FIXED 映射的逻辑，可以从下面拷贝) ...
                VirtAddr::from(addr.align_down(page_size))
            } else {
                aspace
                    .find_free_area(
                        VirtAddr::from(addr.align_down(page_size)),
                        length.align_up(page_size),
                        VirtAddrRange::new(aspace.base(), aspace.end()), // limit
                        page_size,
                    )
                    .ok_or(LinuxError::ENOMEM)?
            };

            // 3. 执行物理内存映射 (这是核心区别！)
            // 我们需要一个新的 aspace 方法：map_physical
            aspace.map_physical(
                start_vaddr,
                paddr, // 使用从 UIO 设备获取的物理地址
                length.align_up(page_size),
                permission_flags.into(), // 转换权限标志
                page_size,
            )?;

            // 4. 成功，返回映射的虚拟地址
            info!(
                "sys_mmap: Mapped UIO device paddr {:#x} to vaddr {:#x}",
                paddr, start_vaddr
            );
            return Ok(start_vaddr.as_usize() as isize);
        }
    }
    // ========================== 【【【 新增逻辑结束 】】】 ==========================

    let page_size = if map_flags.contains(MmapFlags::HUGE_1GB) {
        PageSize::Size1G
    } else if map_flags.contains(MmapFlags::HUGE) {
        PageSize::Size2M
    } else {
        PageSize::Size4K
    };

    let start = addr.align_down(page_size);
    let end = (addr + length).align_up(page_size);
    let aligned_length = end - start;
    debug!(
        "start: {:x?}, end: {:x?}, aligned_length: {:x?}",
        start, end, aligned_length
    );

    let start_addr = if map_flags.contains(MmapFlags::FIXED) {
        if start == 0 {
            return Err(LinuxError::EINVAL);
        }
        let dst_addr = VirtAddr::from(start);
        aspace.unmap(dst_addr, aligned_length)?;
        dst_addr
    } else {
        aspace
            .find_free_area(
                VirtAddr::from(start),
                aligned_length,
                VirtAddrRange::new(aspace.base(), aspace.end()),
                page_size,
            )
            .or(aspace.find_free_area(
                aspace.base(),
                aligned_length,
                VirtAddrRange::new(aspace.base(), aspace.end()),
                page_size,
            ))
            .ok_or(LinuxError::ENOMEM)?
    };

    let populate = if fd == -1 {
        false
    } else {
        !map_flags.contains(MmapFlags::ANONYMOUS)
    };

    aspace.map_alloc(
        start_addr,
        aligned_length,
        permission_flags.into(),
        populate,
        page_size,
    )?;

    if populate {
        let file = File::from_fd(fd)?;
        let file = file.inner();
        let file_size = file.get_attr()?.size() as usize;
        if offset < 0 || offset as usize >= file_size {
            return Err(LinuxError::EINVAL);
        }
        let offset = offset as usize;
        let length = core::cmp::min(length, file_size - offset);
        let mut buf = vec![0u8; length];
        file.read_at(offset as u64, &mut buf)?;
        aspace.write(start_addr, page_size, &buf)?;
    }
    Ok(start_addr.as_usize() as _)
}

pub fn sys_munmap(addr: usize, length: usize) -> LinuxResult<isize> {
    let curr = current();
    let process_data = curr.task_ext().process_data();
    let mut aspace = process_data.aspace.lock();
    let length = align_up_4k(length);
    let start_addr = VirtAddr::from(addr);
    aspace.unmap(start_addr, length)?;
    axhal::arch::flush_tlb(None);
    Ok(0)
}

pub fn sys_mprotect(addr: usize, length: usize, prot: u32) -> LinuxResult<isize> {
    // TODO: implement PROT_GROWSUP & PROT_GROWSDOWN
    let Some(permission_flags) = MmapProt::from_bits(prot) else {
        return Err(LinuxError::EINVAL);
    };
    if permission_flags.contains(MmapProt::GROWDOWN | MmapProt::GROWSUP) {
        return Err(LinuxError::EINVAL);
    }

    let curr = current();
    let process_data = curr.task_ext().process_data();
    let mut aspace = process_data.aspace.lock();
    let length = align_up_4k(length);
    let start_addr = VirtAddr::from(addr);
    aspace.protect(start_addr, length, permission_flags.into())?;

    Ok(0)
}
