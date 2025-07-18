#![no_std]
extern crate alloc;

extern crate axdriver_base;
extern crate axhal;
extern crate axsync;
extern crate lazy_static;

use alloc::string::String;
use alloc::vec::Vec;
use axdriver_base::DeviceType;
use axhal::mem::PhysAddr;
use axsync::Mutex;
use lazy_static::lazy_static;

/// Information about a discovered device, to be passed between modules.
#[derive(Debug, Clone)]
pub struct DiscoveredDeviceInfo {
    pub device_type: DeviceType,
    pub name: String,
    pub pci_bdf: String, // For logging and unique naming, e.g., "00:03.0"
    pub mmio_region: Option<(PhysAddr, usize)>, // Base address and size (physical)
    pub irq_num: Option<usize>,
    // Add other relevant info as needed, like capabilities, transport specific data etc.
}

lazy_static! {
    /// A global list where discovered device info is pushed by axdriver.
    /// axuio (or other modules) can query this list.
    pub static ref DISCOVERED_DEVICES: Mutex<Vec<DiscoveredDeviceInfo>> = Mutex::new(Vec::new());
}

/// Publishes a new discovered device's information.
/// Called by axdriver when it finds and initializes a device.
pub fn publish_device_info(info: DiscoveredDeviceInfo) {
    DISCOVERED_DEVICES.lock().push(info);
}
