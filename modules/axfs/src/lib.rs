//! [ArceOS](https://github.com/arceos-org/arceos) filesystem module.
//!
//! It provides unified filesystem operations for various filesystems.
//!
//! # Cargo Features
//!
//! - `fatfs`: Use [FAT] as the main filesystem and mount it on `/`. This feature
//!   is **enabled** by default.
//! - `devfs`: Mount [`axfs_devfs::DeviceFileSystem`] on `/dev`. This feature is
//!   **enabled** by default.
//! - `ramfs`: Mount [`axfs_ramfs::RamFileSystem`] on `/tmp`. This feature is
//!   **enabled** by default.
//! - `myfs`: Allow users to define their custom filesystems to override the
//!   default. In this case, [`MyFileSystemIf`] is required to be implemented
//!   to create and initialize other filesystems. This feature is **disabled** by
//!   by default, but it will override other filesystem selection features if
//!   both are enabled.
//!
//! [FAT]: https://en.wikipedia.org/wiki/File_Allocation_Table
//! [`MyFileSystemIf`]: fops::MyFileSystemIf

#![cfg_attr(all(not(test), not(doc)), no_std)]
#![feature(doc_auto_cfg)]

#[macro_use]
extern crate lazy_static;
extern crate alloc;
#[macro_use]
extern crate log;

mod dev;
mod fs;
mod mounts;
mod root;

pub mod api;
pub mod fops;
pub use root::{CURRENT_DIR, CURRENT_DIR_PATH};

use alloc::sync::Arc;
use axdriver::{AxDeviceContainer, prelude::*};
use axfs_devfs::DeviceFileSystem;
use spin::Mutex; // 使用 spinlock

lazy_static! {
    static ref DEVFS_CONTAINER: Mutex<Option<Arc<DeviceFileSystem>>> = Mutex::new(None);
}

pub mod DEVFS {
    use super::*;
    pub fn get() -> Option<Arc<DeviceFileSystem>> {
        DEVFS_CONTAINER.lock().as_ref().cloned()
    }
}

/// Initializes filesystems by block devices.
pub fn init_filesystems(mut blk_devs: AxDeviceContainer<AxBlockDevice>) {
    info!("Initialize filesystems...");

    let dev = blk_devs.take_one().expect("No block device found!");
    info!("  use block device 0: {:?}", dev.device_name());
    self::root::init_rootfs(self::dev::Disk::new(dev));
}

pub fn set_devfs_instance(instance: Arc<DeviceFileSystem>) {
    let mut devfs_lock = DEVFS_CONTAINER.lock();
    if devfs_lock.is_some() {
        panic!("The global DEVFS instance has already been set.");
    }
    *devfs_lock = Some(instance);
}
