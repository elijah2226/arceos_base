#![no_std]
#![no_main]

// 假设你的库叫 nimbos 或 user_lib
// 如果是 nimbos, 就用 extern crate nimbos;
#[macro_use]
extern crate user_lib; 

// 从你的库中导入需要的系统调用封装
use user_lib::{exit, vfork, wait, getpid};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("vfork_test: test starting...");

    // 1. 定义一个将被共享和修改的变量
    let mut shared_var: usize = 100;
    println!("vfork_test: In parent, before vfork, shared_var = {}", shared_var);

    // 2. 调用 vfork
    let pid = vfork();

    if pid == 0 {
        // --- 子进程代码块 ---
        // 这部分代码在父进程的地址空间中执行，但父进程是暂停的

        println!(
            "vfork_test: In child (pid {}), shared_var = {}. Modifying it.",
            getpid(),
            shared_var
        );
        
        // 3. 子进程修改共享变量
        shared_var += 50;

        println!(
            "vfork_test: In child, after modification, shared_var = {}. Exiting now.",
            shared_var
        );

        // 4. 子进程必须调用 exit() 或 exec() 来唤醒父进程。
        //    使用 exit() 是测试 vfork 语义的一种方式。
        exit(0);

    } else if pid > 0 {
        // --- 父进程代码块 ---
        println!("vfork_test: In parent (pid {}), woken up. Child pid is {}.", getpid(), pid);

        // ... (对 shared_var 的检查不变) ...

        // 6. 调用 wait 回收子进程资源，并验证退出码
        let mut exit_code: i32 = -1;
        // --- 这就是修改后的代码 ---
        let wait_pid = wait(Some(&mut exit_code));
        // --- 修改结束 ---

        if wait_pid == pid as isize {
            println!("vfork_test: wait() returned correct pid: {}.", wait_pid);
        } else {
            println!(
                "vfork_test: FAILED - wait() returned {}, expected {}.",
                wait_pid, pid
            );
            return -1;
        }

        if exit_code == 0 {
             println!("vfork_test: SUCCESS - child exited with correct code: 0.");
        } else {
            println!(
                "vfork_test: FAILED - child exited with code {}, expected 0.",
                exit_code
            );
            return -1;
        }

        println!("vfork_test: test passed!");
        0 // 所有检查都通过，测试成功

    } else {
        // --- vfork 调用失败 ---
        println!("vfork_test: FAILED - vfork system call failed, returned {}.", pid);
        -1 // 测试失败
    }
}