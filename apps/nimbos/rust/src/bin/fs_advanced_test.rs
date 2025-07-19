
#![no_std]
#![no_main]

extern crate user_lib; 

use user_lib::{
    // 文件操作
    open, close, write, read, unlink,
    // 新的系统调用
    symlink, readlink, chmod, chown,
    // 进程操作
    exit, access, mknod, mkdir, rmdir,
    c_str_lit, test_case,
};
use user_lib::{OpenFlags, Stat, stat, println}; 
use core::ffi::c_char; 
use linux_raw_sys::general::{AT_FDCWD, S_IFMT, S_IFREG, S_IFIFO};


// === 测试 1: symlink 和 readlink 的基本功能 ===
fn symlink_readlink_basic() -> bool {  
    let target_path = c_str_lit!("/tmp/test_target_file.txt");
    let link_path = c_str_lit!("/tmp/my_link");
    let content = b"hello world from target file";
    let mut success = true;

    // 1. 创建目标文件并写入内容
    let fd =  open(target_path, OpenFlags::CREATE | OpenFlags::WRONLY);
    if fd < 0 {
        println!("Failed to create target file!");
        success = false;
    } else {
        println!("Successfully created target file!");
        write(fd as usize, content);
    }
    close(fd as usize);

    if success {
        let ret = symlink(target_path, link_path);
        if ret < 0 {
            if ret == -38 || ret == -95 {
                println!("symlink() syscall not supported, skipping test, errno={}", ret);
                return true;
            } else {
                println!("symlink failed: {}", ret);
                success = false;
            }
        }
        
    }

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
        close(fd_link as usize);
    }
    success
}

// === 测试 2: chmod 功能 ===
fn chmod_functionality() -> bool{
    let file_path = c_str_lit!("/tmp/chown_test_file.txt");
    let new_uid = 1001;
    let new_gid = 2002;
    let mut success = true;

    // 1. 创建文件
    let fd = open(file_path, OpenFlags::CREATE | OpenFlags::WRONLY);
    if fd < 0 {
        println!("Failed to create file for chown test!");
        success = false;
    } else {
        close(fd as usize);
    }

    // 2. 修改所有者
    if success {
        let ret = chmod(file_path, 0o644);
        if ret != 0 {
            println!("symlink() syscall not supported, skipping test, errno={}", ret);
            if ret == -38 || ret == -95 {
                return true;
            }else{
                success = false;
            }
        }
    }

    if success{
        let fd_link = open(file_path,OpenFlags::CREATE | OpenFlags::WRONLY);
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

    success
}

// === 测试 3: chown 功能 ===
fn chown_functionalit() ->bool{
    let file_path = c_str_lit!("/tmp/chown_test_file.txt");
    let new_uid = 1001;
    let new_gid = 2002;
    let mut success = true;

    // 1. 创建文件
    let fd = open(file_path, OpenFlags::CREATE | OpenFlags::WRONLY);
    if fd < 0 {
        println!("Failed to create file for chown test!");
        success = false;
    } else {
        close(fd as usize);
    }

    // 2. 修改所有者
    if success {
        let ret = chown(file_path, new_uid, new_gid);
        if ret != 0 {
            println!("symlink() syscall not supported, skipping test, errno={}", ret);
            if ret == -38 || ret == -95 {
                return true;
            }else{
                success = false;
            }
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
    success
}



// === 新增测试: access 功能 ===
fn access_functionality() -> bool {
    let test_file = c_str_lit!("/tmp/access_test_file.txt");
    let mut success = true;

    // 创建测试文件，rwx------ (0o700)
    let fd = open(test_file, OpenFlags::CREATE | OpenFlags::WRONLY);
    if fd < 0 {
        println!("Failed to create file for access test!");
        return false;
    }
    close(fd as usize);

    // 测试 1: F_OK (只检查文件是否存在)
    let ret = access(test_file, 0); // F_OK == 0
    if ret != 0 {
        if ret == -38 || ret == -95 { // Not supported
            println!("access(F_OK) not supported, skipping test, errno={}", ret);
            unsafe { unlink(test_file); }
            return true;
        } else {
            println!("access(F_OK) failed unexpectedly! ret={}", ret);
            success = false;
        }
    } else {
        println!("access(F_OK) PASSED!");
    }

    // 测试 2: R_OK (读权限)
    if success {
        let ret = access(test_file, 4); // R_OK
        if ret != 0 {
            if ret == -38 || ret == -95 {
                println!("access(R_OK) not supported, skipping test, errno={}", ret);
                unsafe { unlink(test_file); }
                return true;
            }
            else { success = false; println!("access(R_OK) failed unexpectedly! ret={}", ret); }
        } else { println!("access(R_OK) PASSED!"); }
    }
    
    // 测试 3: W_OK (写权限)
    if success {
        let ret = access(test_file, 2); // W_OK
        if ret != 0 {
            if ret == -38 || ret == -95 { 
                println!("access(W_OK) not supported, skipping test, errno={}", ret);
                unsafe { unlink(test_file); }
                return true;
             }
            else { success = false; println!("access(W_OK) failed unexpectedly! ret={}", ret); }
        } else { println!("access(W_OK) PASSED!"); }
    }
    
    // 测试 4: X_OK (执行权限)
    let ret_chmod_test = chmod(test_file, 0o777); // 尝试给它一个执行权限
    if ret_chmod_test == -38 || ret_chmod_test == -95 { 
        println!("access_functionality: chmod syscall not supported (errno={}), skipping X_OK test.", ret_chmod_test);
        unsafe { unlink(test_file); }
        return true; 
    } else if ret_chmod_test != 0 {
        println!("access_functionality: chmod(0o777) failed unexpectedly! ret={}", ret_chmod_test);
        unsafe { unlink(test_file); }
        return false;
    } else {
        println!("access_functionality: Chmod file to 0o777 for tests (may be a dummy operation on FATFS).");
    }

    if success {
        let ret = access(test_file, 1); // X_OK
        if ret != 0 {
            if ret == -38 || ret == -95 { 
                println!("access(X_OK) not supported, skipping test, errno={}", ret);
                unsafe { unlink(test_file); }
                return true;
             }
            else { success = false; println!("access(X_OK) failed unexpectedly! ret={}", ret); }
        } else { println!("access(X_OK) PASSED!"); }
    }

    // 清理
    unsafe { unlink(test_file); }

    success
}


// === 新增测试: mknodat 功能 ===
fn mknod_functionality() -> bool {
    let test_dir = c_str_lit!("/tmp/mknod_test_dir");
    let test_file_path = c_str_lit!("/tmp/mknod_test_file");
    let test_fifo_path = c_str_lit!("/tmp/mknod_test_fifo");
    let mut success = true;

    // 创建 /tmp/mknod_test_dir 目录
    let ret_mkdir = mkdir(test_dir); // 假设 user_lib 有 mkdir 封装
    if ret_mkdir != 0 {
        if ret_mkdir == -38 || ret_mkdir == -95 {
            println!("mkdir() not supported, skipping mknod test.");
            return true; // 跳过测试
        } else {
            println!("Failed to create /tmp/mknod_test_dir for mknod test! ret={}", ret_mkdir);
            return false;
        }
    } else {
        println!("Created test directory: {}", "/tmp/mknod_test_dir");
    }

    // 测试 1: 创建普通文件 (S_IFREG)
    // S_IFREG 通常是 0x8000
    let file_mode = 0o644; // 权限 rw-r--r--
    let ret = mknod(test_file_path, S_IFREG | file_mode); 
    if ret != 0 {
        if ret == -38 || ret == -95 { /* skip */ } // 如果是 ENOSYS 或 EOPNOTSUPP，则跳过
        else { println!("mknod(S_IFREG) failed! ret={}", ret); success = false; }
    } else {
        // 验证文件类型
        let mut stat_buf = Stat::new();
        if stat(test_file_path, &mut stat_buf) != 0 { success = false; println!("stat failed after mknod"); }
        else if (stat_buf.mode & S_IFMT) != S_IFREG { 
            println!("mknod(S_IFREG) type mismatch! Expected {:#x}, got {:#x}", S_IFREG, stat_buf.mode & S_IFMT);
            success = false;
        } else {
            println!("mknod(S_IFREG) PASSED!");
        }
    }
    
    // 测试 2: 创建命名管道 (S_IFIFO) - 许多文件系统（如 FAT）不支持 FIFO
    if success {
        let fifo_mode = 0o666;
        let ret = mknod(test_fifo_path, S_IFIFO | fifo_mode); 
        if ret != 0 {
            if ret == -38 || ret == -95 { /* skip */ }
            else { println!("mknod(S_IFIFO) failed! ret={}", ret); success = false; }
        } else {
            let mut stat_buf = Stat::new();
            stat(test_fifo_path, &mut stat_buf);
            if (stat_buf.mode & S_IFMT) != S_IFIFO { 
                println!("mknod(S_IFIFO) type mismatch!"); 
                success = false; 
            } else {
                println!("mknod(S_IFIFO) PASSED!");
            }
        }
    }

    // 清理
    unsafe {
        unlink(test_file_path); // 如果成功创建了文件
        unlink(test_fifo_path); // 如果成功创建了 FIFO
        // rmdir(test_dir); // 删除测试目录
    }

    success
}







#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("\n======== Advanced FS Syscall Test Suite ========");

    // === 测试 1: symlink 和 readlink 的基本功能 ===
    test_case!("symlink_readlink_basic", symlink_readlink_basic);

     // === 测试 2: chmod 功能 ===
    test_case!("chmod_functionalit", chmod_functionality);

    // === 测试 2: chown 功能 ===
    test_case!("chown_functionalit", chown_functionalit);


    test_case!("access_functionality", access_functionality);
    test_case!("mknod_functionality", mknod_functionality);
    
    println!("======== Advanced FS Syscall Test Suite PASSED ========");
    0
}