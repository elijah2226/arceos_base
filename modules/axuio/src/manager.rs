// /starry/.arceos/modules/axuio/src/manager.rs

use super::device::{UioDevice, UioIrq, UioMemoryRegion};
use crate::create_device_file;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use axerrno::AxResult;
use axhal::arch::{disable_irqs, enable_irqs, irqs_enabled};
use axsync::Mutex;
use axtask::WaitQueue;
use lazy_static::lazy_static;

// --- 【【【修改：全局设备列表，现在也用于中断分发】】】 ---
lazy_static! {
    static ref UIO_DEVICES: Mutex<Vec<Arc<UioDevice>>> = Mutex::new(Vec::new());
}

/// 注册一个新的 UIO 设备。
///
/// 这是 UIO 子系统的主要入口点。设备驱动程序调用此函数来
/// 将其管理的硬件暴露为 UIO 设备。
///
/// 此函数会：
/// 1. 为设备分配一个唯一的 ID。
/// 2. 如果需要，注册并启用其中断处理程序。
/// 3. 将设备信息存储在全局列表中。
/// 4. 触发在 DEVFS 中创建对应的 `/dev/uioX` 文件节点。
///
/// # 返回
/// 成功时返回设备 ID，失败时返回错误。
// --- 【【【修改：返回值类型，并调用 create_device_file】】】 ---
pub fn register_device(
    name: String,
    version: String,
    mem_regions: Vec<UioMemoryRegion>,
    irq_num: Option<usize>,
) -> AxResult<usize> {
    let mut devices = UIO_DEVICES.lock();
    let id = devices.len();

    let irq = if let Some(irq_num) = irq_num {
        let handler: fn() = match id {
            0 => uio_irq_handler_0,
            1 => uio_irq_handler_1,
            2 => uio_irq_handler_2,
            _ => {
                return axerrno::ax_err!(Unsupported, "UIO device limit reached for IRQ handlers.");
            }
        };

        if !axhal::irq::register_handler(irq_num, handler) {
            return axerrno::ax_err!(AlreadyExists, "Failed to register IRQ handler");
        }
        axhal::irq::set_enable(irq_num, true);
        info!("UIO device {} registered IRQ {} with handler.", id, irq_num);

        Some(UioIrq {
            irq_num,
            wait_queue: Arc::new(WaitQueue::new()),
            count: Arc::new(Mutex::new(0)),
        })
    } else {
        None
    };

    let device = Arc::new(UioDevice {
        id,
        name,
        version,
        mem_regions,
        irq,
    });
    devices.push(device.clone());
    // 释放锁，因为 create_device_file 可能需要获取其他锁
    drop(devices);

    // --- 【【【新增】】】 注册成功后，创建对应的设备文件 ---
    create_device_file(id)?;

    Ok(id)
}

/// 根据 ID 获取设备 (内部使用)
pub(crate) fn get_device(id: usize) -> Option<Arc<UioDevice>> {
    UIO_DEVICES.lock().get(id).cloned()
}

fn uio_irq_handler_0() {
    uio_irq_dispatcher(0);
}
fn uio_irq_handler_1() {
    uio_irq_dispatcher(1);
}
fn uio_irq_handler_2() {
    uio_irq_dispatcher(2);
}

fn uio_irq_dispatcher(device_id: usize) {
    if let Some(device) = get_device(device_id) {
        if let Some(irq) = &device.irq {
            *irq.count.lock() += 1;
            irq.wait_queue.notify_all(true); // 使用 notify_all(true) 来唤醒任务并强制调度
            trace!(
                "UIO IRQ for device {} (irq {}) handled.",
                device.id, irq.irq_num
            );
        }
    }
}
