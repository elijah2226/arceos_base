// arceos-main/src/main.rs

#![no_std]
#![no_main]

use axstd::println;

extern crate alloc;
extern crate axlinux;
extern crate axns;

use alloc::sync::Arc;
use axfs_devfs::DeviceFileSystem;

/// This is the main function for the unikernel application.
/// It must be named `main` and have the C ABI to be called by `axruntime`.
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    println!("[arceos-main] Application 'main' function started!");

    {
        println!("[arceos-main] Initializing UIO subsystem...");
        axuio::init();
        // 这个调用现在是完全安全的，因为它依赖的 DEVFS 已经初始化完毕。
        // axuio::register_discovered_devices();
        // axuio::test_register_dummy_device();
        println!("[arceos-main] Initializing UIO HPET device...");
        if let Err(e) = axuio::register_hpet_device() {
            println!(
                "[arceos-main] Failed to initialize UIO HPET device: {:?}",
                e
            );
        }

        axtask::spawn(|| {
            loop {
                axtask::sleep(core::time::Duration::from_secs(5)); // 等待 5 秒
                println!("[kernel-test] Manually triggering dummy UIO IRQ...");
                axuio::uio_irq_dispatcher(0);
            }
        });
    }

    // --- 【【【阶段 3: 启动用户态进程】】】 ---
    #[cfg(feature = "linux_compat")]
    {
        println!("[arceos-main] Handing control to Linux...");
        axlinux::init();
    }

    #[cfg(not(feature = "linux_compat"))]
    {
        println!("[arceos-main] Kernel initialized. Halting.");
        loop {
            axhal::cpu::spin_loop_hint();
        }
    }
}
