// /starry/.arceos/modules/axuio/src/lib.rs

#![no_std]

#[macro_use]
extern crate axlog;
extern crate alloc;
extern crate axerrno; // 【【【新增】】】

mod device;
mod file;
mod manager;

pub use device::UioMemoryRegion;
pub use manager::register_device;

use axerrno::{AxError, AxResult};

/// UIO 模块的全局初始化函数
pub fn init() {
    info!("axuio module initialized.");
}

// /starry/.arceos/modules/axuio/src/lib.rs (最终版)
pub fn create_device_file(device_id: usize) -> AxResult {
    use alloc::sync::Arc;
    use file::UioDeviceFile;

    // 1. 使用我们刚刚公开的全局 DEVFS 实例
    if let Some(devfs_instance) = axfs::DEVFS::get() {
        let uio_node = Arc::new(UioDeviceFile::new(device_id)?);

        let device_name = match device_id {
            0 => "uio0",
            1 => "uio1",
            _ => return axerrno::ax_err!(NoMemory, "device id too large"),
        };

        // 2. 调用 add 方法！调用链路打通！
        devfs_instance.add(device_name, uio_node);

        info!("Successfully registered UIO device at /dev/{}", device_name);
        Ok(())
    } else {
        axerrno::ax_err!(NotFound, "DEVFS is not initialized or feature not enabled")
    }
}
