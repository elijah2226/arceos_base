// /arceos/apps/nimbos/rust/src/bin/fs_advanced_test.rs

#![no_std]
#![no_main]

// 使用你项目中的用户库，它可能是 `user_lib` 或 `nimbos`
// 请根据你的实际情况修改
extern crate user_lib; 

use user_lib::{
    // 文件操作
    open, close, write, read, unlink,
    // 新的系统调用
    symlink, readlink, chmod, chown,
    // 进程操作
    exit,
};
use user_lib::{OpenFlags, Stat, stat, println}; 

// 一个辅助宏，用于简化测试断言和日志输出
macro_rules! test_case {
    ($name:expr, $test_block:block) => {
        println!("--- Test case [ {} ] starting ---", $name);
        if $test_block {
            println!("--- Test case [ {} ] PASSED ---\n", $name);
        } else {
            println!("--- Test case [ {} ] FAILED! ---\n", $name);
            // 测试失败，立即退出
            exit(-1);
        }
    };
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("\n======== Advanced FS Syscall Test Suite ========");

    // === 测试 1: symlink 和 readlink 的基本功能 ===
    test_case!("symlink_readlink_basic", {
        let target_path = "/test_target_file.txt";
        let link_path = "/my_link";
        let content = b"hello world from target file";
        let mut success = true;

        // 1. 创建目标文件并写入内容
        let fd = open(target_path, OpenFlags::CREATE | OpenFlags::WRONLY);
        if fd < 0 {
            println!("Failed to create target file!");
            success = false;
        } else {
            write(fd as usize, content);
            close(fd as usize);
        }

        // 2. 创建符号链接
        if success {
            let ret = symlink(target_path, link_path);
            if ret != 0 {
                println!("symlink() syscall failed, ret = {}", ret);
                success = false;
            }
        }
        
        // 3. 读取符号链接，验证其内容
        if success {
            let mut buf = [0u8; 128];
            let len = readlink(link_path, &mut buf);
            if len < 0 {
                println!("readlink() syscall failed, ret = {}", len);
                success = false;
            } else {
                let link_content = core::str::from_utf8(&buf[..len as usize]).unwrap();
                if link_content != target_path {
                    println!("Readlink content mismatch! Expected '{}', got '{}'", target_path, link_content);
                    success = false;
                }
            }
        }
        
        // 4. 通过符号链接读取文件内容
        if success {
            let fd_link = open(link_path, OpenFlags::RDONLY);
            if fd_link < 0 {
                println!("Failed to open file via symlink!");
                success = false;
            } else {
                let mut read_buf = [0u8; 128];
                let read_len = read(fd_link as usize, &mut read_buf);
                close(fd_link as usize);
                if read_len as usize != content.len() || &read_buf[..read_len as usize] != content {
                    println!("Content read via symlink is incorrect!");
                    success = false;
                }
            }
        }

        // 清理
        unlink(target_path);
        unlink(link_path);
        
        success
    });

    // === 测试 2: chmod 功能 ===
    test_case!("chmod_functionality", {
        let file_path = "/chmod_test_file";
        let mut success = true;

        // 1. 创建文件
        let fd = open(file_path, OpenFlags::CREATE | OpenFlags::WRONLY);
        if fd < 0 {
            println!("Failed to create file for chmod test!");
            success = false;
        } else {
            close(fd as usize);
        }

        // 2. 设置一个特定的权限，例如 rwx------ (0o700)
        if success {
            let mode = 0o700;
            if chmod(file_path, mode) != 0 {
                println!("chmod() syscall failed!");
                success = false;
            }
        }

        // 3. 使用 stat 获取文件信息并验证权限
        if success {
            let mut stat_buf = Stat::new();
            if stat(file_path, &mut stat_buf) != 0 {
                println!("stat() syscall failed!");
                success = false;
            } else {
                // POSIX 权限位在 mode 的低12位
                let perms = stat_buf.mode & 0o777;
                if perms != 0o700 {
                    println!("Chmod verification failed! Expected mode {:#o}, got {:#o}", 0o700, perms);
                    success = false;
                }
            }
        }

        // 4. 再改成另一个权限，例如 r--r--r-- (0o444)
        if success {
            let mode = 0o444;
            chmod(file_path, mode);
            let mut stat_buf = Stat::new();
            stat(file_path, &mut stat_buf);
            let perms = stat_buf.mode & 0o777;
            if perms != 0o444 {
                println!("Second chmod verification failed! Expected mode {:#o}, got {:#o}", 0o444, perms);
                success = false;
            }
        }

        // 清理
        unlink(file_path);
        
        success
    });
    
    // === 测试 3: chown 功能 ===
    test_case!("chown_functionality", {
        let file_path = "/chown_test_file";
        let new_uid = 1001;
        let new_gid = 2002;
        let mut success = true;

        // 1. 创建文件
        let fd = open(file_path, OpenFlags::CREATE);
        if fd < 0 {
            println!("Failed to create file for chown test!");
            success = false;
        } else {
            close(fd as usize);
        }

        // 2. 修改所有者
        if success {
            if chown(file_path, new_uid, new_gid) != 0 {
                println!("chown() syscall failed!");
                success = false;
            }
        }

        // 3. 使用 stat 获取文件信息并验证所有者
        if success {
            let mut stat_buf = Stat::new();
            if stat(file_path, &mut stat_buf) != 0 {
                println!("stat() syscall failed!");
                success = false;
            } else {
                if stat_buf.uid != new_uid {
                    println!("Chown UID verification failed! Expected {}, got {}", new_uid, stat_buf.uid);
                    success = false;
                }
                if stat_buf.gid != new_gid {
                    println!("Chown GID verification failed! Expected {}, got {}", new_gid, stat_buf.gid);
                    success = false;
                }
            }
        }
        
        // 清理
        unlink(file_path);
        
        success
    });
    
    // 可以在这里添加更多边界情况的测试，例如对不存在的文件操作等。

    println!("======== Advanced FS Syscall Test Suite PASSED ========");
    exit(0);
}