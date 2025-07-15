// arceos-main/src/main.rs

#![no_std]
#![no_main]

use axstd::println;

extern crate axlinux;
extern crate axns;

/// This is the main function for the unikernel application.
/// It must be named `main` and have the C ABI to be called by `axruntime`.
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    axuio::init();

    println!("[arceos-main] Application 'main' function started!");
    // ...
    #[cfg(feature = "linux_compat")]
    {
        // Initialize the Linux compatibility layer.
        println!("[arceos-main] Linux compatibility layer initialized.");
        axlinux::init();
    }
    #[cfg(feature = "fp_simd")]
    {
        // Initialize the Linux compatibility layer.
        println!("[arceos-main] FP SIMD layer initialized.");
    }
    #[cfg(feature = "fp_simd")]
    {
        // Initialize the Linux compatibility layer.
        println!("[arceos-main] FP SIMD layer initialized.");
    }
    #[cfg(feature = "uio")]
    {
        // Initialize the UIO layer.
        axuio::init();
        println!("[arceos-main] UIO layer initialized.");
    }
    #[cfg(feature = "axfeat/linux_normal_mode")]
    {
        // Start the init process.
        axlinux::start_init_process();
        println!("[arceos-main] Init process started.");
    }

    // 你也可以添加一个 "not" 检查，用于验证特性未被启用的情况。
    #[cfg(not(feature = "linux_compat"))]
    {
        println!("[arceos-main] CHECK NOTICE: 'linux_compat' feature is DISABLED.");
        axlinux::init();
    }
    // --- 检验代码结束 ---

    println!("[arceos-main] Application 'main' function finished.");
}
