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

    // --- 【【【阶段 1: 核心文件系统初始化】】】 ---
    println!("[arceos-main] Initializing all filesystems with a single call...");

    // 【【【修改】】】 相信 axfs::api::init() 会根据 Cargo features 自动完成所有工作：
    // 1. 初始化 RamFS 并挂载到 "/"
    // 2. 创建 /dev 目录
    // 3. 创建 DeviceFileSystem 实例，设置全局变量，并挂载到 "/dev"
    // axfs::api::init().expect("Failed to initialize filesystems");

    // --- 【【【阶段 2: 依赖服务初始化】】】 ---
    // 此刻，DEVFS 已经由 axfs::api::init() 准备就绪。
    {
        println!("[arceos-main] Initializing UIO subsystem...");
        axuio::init();
        // 这个调用现在是完全安全的，因为它依赖的 DEVFS 已经初始化完毕。
        axuio::test_register_dummy_device();
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
