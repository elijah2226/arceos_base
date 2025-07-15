// /starry/.arceos/modules/axuio/src/manager.rs

use super::device::{UioDevice, UioIrq, UioMemoryRegion};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use axhal::arch::{disable_irqs, enable_irqs, irqs_enabled};
use axsync::Mutex;
use axtask::WaitQueue;
use lazy_static::lazy_static;

// --- 【【【修改：全局设备列表，现在也用于中断分发】】】 ---
lazy_static! {
    static ref UIO_DEVICES: Mutex<Vec<Arc<UioDevice>>> = Mutex::new(Vec::new());
}

/// 注册一个新的 UIO 设备
pub fn register_device(
    name: String,
    version: String,
    mem_regions: Vec<UioMemoryRegion>,
    irq_num: Option<usize>,
) -> Result<usize, &'static str> {
    let mut devices = UIO_DEVICES.lock();
    let id = devices.len();

    let irq = if let Some(irq_num) = irq_num {
        // --- 【【【最终版：使用正确的函数指针】】】 ---
        let handler: fn() = match id {
            0 => uio_irq_handler_0,
            1 => uio_irq_handler_1,
            2 => uio_irq_handler_2,
            _ => return Err("UIO device limit reached for IRQ handlers."),
        };

        // --- 【【【最终版：使用正确的注册和使能函数】】】 ---
        if !axhal::irq::register_handler(irq_num, handler) {
            return Err("Failed to register IRQ handler");
        }
        axhal::irq::set_enable(irq_num, true);

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
    devices.push(device);
    Ok(id)
}

/// 根据 ID 获取设备
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

/// 通用中断分发逻辑
fn uio_irq_dispatcher(device_id: usize) {
    if let Some(device) = get_device(device_id) {
        if let Some(irq) = &device.irq {
            // 我们不需要在这里屏蔽/使能中断，因为CPU在进入中断处理时会自动屏蔽，
            // 并且我们也不需要单独屏蔽这个 IRQ line。

            *irq.count.lock() += 1;
            irq.wait_queue.notify_all(true);

            trace!(
                "UIO IRQ for device {} (irq {}) handled.",
                device.id, irq.irq_num
            );

            // 我们不需要调用 EOI，底层框架会处理。
        }
    }
}
