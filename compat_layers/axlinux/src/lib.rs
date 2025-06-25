// /starry/.arceos/compat_layers/axlinux/src/lib.rs

// 【修改1】移除 #[no_main]，因为它现在是一个库
#![no_std]

#[macro_use]
extern crate axlog;
extern crate alloc;

// 在文件顶部添加这两行
use alloc::string::ToString;

// --- 【【【在这里加入所有缺失的依赖声明】】】 ---
// This brings the crate names into the root namespace of the `axlinux` crate,
// making them available to all submodules like `entry`, `mm`, etc.
use axhal;
// use axsync;
// 你可能还需要为其他报错的 crate 也加上，比如 axprocess, axfs...
// 最好把你 Cargo.toml 里所有 optional=false 的依赖都在这里声明一下
// use axconfig;
// use axerrno;
#[cfg(feature = "task")]
use axprocess;
// #[cfg(feature = "task")]
// use axsignal;
#[cfg(feature = "task")]
use axtask;
// use bitflags;

// 导出需要被外部（比如测试代码）访问的模块
// 如果模块是私有的，就不需要 pub
mod entry;
mod mm;
mod syscall;

// use alloc::string::ToString;

// 【修改2】提供一个公共的、可被 arceos-main 调用的初始化函数
/// Initializes the Linux compatibility layer.
/// This function sets up the initial process and starts the main loop,
/// either running tests or launching an init process.
pub fn init() -> ! {
    // 无论在哪种模式下，都需要创建 PID=1 的 init 进程结构。
    axprocess::Process::new_init(axtask::current().id().as_u64() as _).build();
    info!("[axlinux] init process structure created.");

    // 使用 #[cfg] 进行条件编译，这部分逻辑和原来完全一样
    #[cfg(not(feature = "normal_mode"))]
    run_tests();

    #[cfg(feature = "normal_mode")]
    start_init_process();

    // 无论是哪条路径，都不会返回
    unreachable!();
}

// 【修改3】将原来的 main 函数内容改造成库的私有函数
// 并且它们不再需要 #[no_mangle] 或 #[unsafe]

/// Test mode execution logic.
#[cfg(not(feature = "normal_mode"))]
fn run_tests() -> () {
    info!("[axlinux] Running in TEST mode.");
    let testcases = option_env!("AX_TESTCASES_LIST")
        .unwrap_or_else(|| "AX_TESTCASES_LIST not set, no tests to run.")
        .split(',')
        .filter(|&x| !x.is_empty());

    let mut count = 0;
    for testcase in testcases {
        count += 1;
        let Some(args) = shlex::split(testcase) else {
            error!("Failed to parse testcase: {:?}", testcase);
            continue;
        };
        if args.is_empty() {
            continue;
        }
        info!("Running user task [{}]: {:?}", count, args);
        let exit_code = entry::run_user_app(&args, &[]);
        info!("User task {:?} exited with code: {:?}", args, exit_code);
    }

    if count == 0 {
        warn!("[axlinux] No testcases were executed.");
    }

    info!("[axlinux] All tests finished, shutting down.");
    axhal::misc::terminate(); // 所有测试跑完后关机
}

/// Normal startup mode execution logic.
#[cfg(feature = "normal_mode")]
fn start_init_process() -> ! {
    info!("[axlinux] Running in NORMAL mode.");
    const INIT_PATH: &str = "/bin/sh";
    let args = shlex::split(INIT_PATH).expect("Failed to parse init path");
    let envs = ["PATH=/bin:/usr/bin".to_string(), "PWD=/".to_string()];

    info!("Starting init process: {:?} with envs {:?}", args, envs);
    if entry::run_user_app(&args, &envs).is_none() {
        panic!("Failed to start init process!");
    }

    info!("[axlinux] Init process launched. Entering kernel idle loop.");
    loop {
        axtask::yield_now();
    }
}
