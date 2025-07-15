// /starry/.arceos/modules/axuio/src/device.rs

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use axsync::Mutex;
use axhal::mem::PhysAddr;
use axtask::WaitQueue;

#[derive(Debug, Clone, Copy)]
pub struct UioMemoryRegion {
    pub paddr: PhysAddr,
    pub size: usize,
}

pub(crate) struct UioIrq { // `pub(crate)` 表示只在 axuio 模块内部可见
    pub(crate) irq_num: usize,
    pub(crate) wait_queue: Arc<WaitQueue>,
    pub(crate) count: Arc<Mutex<u32>>,
}

pub struct UioDevice {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) mem_regions: Vec<UioMemoryRegion>,
    pub(crate) irq: Option<UioIrq>,
}