#![no_std]
#![feature(linkage)]

#[macro_use]
pub mod console;

mod arch;
mod lang_items;
mod syscall;
mod time;

pub use time::*;
use bitflags::bitflags;
use core::ffi::c_char;
use linux_raw_sys::general::AT_FDCWD;

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0o0000;
        const WRONLY = 0o0001;
        const RDWR = 0o0002;
        const CREATE = 0o0100;
        const TRUNC = 0o0200;
        const EXCL = 0o0400;
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    exit(main());
}

#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot find main!");
}

#[macro_export]
macro_rules! c_str_lit {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}


#[macro_export]
macro_rules! test_case {
    ($name:expr, $test_fn:expr) => { 
        let test_result = $test_fn(); 
        
        if test_result {
            println!("--- Test case [ {} ] PASSED ---\n", $name);
        } else {
            println!("--- Test case [ {} ] FAILED! ---\n", $name);
            exit(-1);
        }
    };
}

use syscall::*;

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code)
}

pub fn sched_yield() -> isize {
    sys_yield()
}

pub fn getpid() -> isize {
    sys_getpid()
}

pub fn fork() -> isize {
    sys_fork()
}

pub fn vfork() -> isize {
    sys_vfork()
}

pub fn exec(path: &str) -> isize {
    sys_exec(path)
}

pub fn waitpid(pid: isize, exit_code: Option<&mut i32>, options: u32) -> isize {
    let exit_code_ptr = exit_code.map(|e| e as _).unwrap_or(core::ptr::null_mut());
    sys_waitpid(pid, exit_code_ptr, options)
}

pub fn wait(exit_code: Option<&mut i32>) -> isize {
    waitpid(-1, exit_code, 0)
}

pub fn thread_spawn(entry: fn(usize) -> i32, arg: usize) -> isize {
    use core::sync::atomic::{AtomicUsize, Ordering};
    const MAX_THREADS: usize = 16;
    const THREAD_STACK_SIZE: usize = 4096 * 4; // 16K
    static mut THREAD_STACKS: [[u8; THREAD_STACK_SIZE]; MAX_THREADS] =
        [[0; THREAD_STACK_SIZE]; MAX_THREADS];
    static THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

    let thread_id = THREAD_COUNT.fetch_add(1, Ordering::AcqRel);
    let newsp = unsafe { THREAD_STACKS[thread_id].as_ptr_range().end as usize };
    sys_clone(entry, arg, newsp)
}

pub fn open(path: *const c_char, flags: OpenFlags) -> isize {
    syscall::sys_open(path, flags.bits())
}

pub fn close(fd : usize) -> isize {
    syscall::sys_close(fd)
}

/// Removes a file. To remove a directory, use `rmdir`.
pub fn unlink(path: *const c_char) -> isize {
    syscall::sys_unlink(path)
}

/// Removes an empty directory.
pub fn rmdir(path: *const c_char) -> isize {
    syscall::sys_rmdir(path)
}

pub fn symlink(target: *const c_char, linkpath:  *const c_char) -> isize {
    syscall::sys_symlink(target, linkpath)
}

pub fn readlink(path:  *const c_char, buf: &mut [u8]) -> isize {
    syscall::sys_readlink(path, buf)
}

pub fn chmod(path:*const c_char, mode: u32) -> isize {
    syscall::sys_chmod(path, mode)
}

pub fn chown(path: *const c_char, uid: u32, gid: u32) -> isize {
    syscall::sys_chown(path, uid, gid)
}

pub fn access(path: *const c_char, mode: u32) -> isize {
    syscall::sys_access(path, mode)
}

pub fn mknod(path: *const c_char, mode: u32) -> isize {
    syscall::sys_mknodat(AT_FDCWD, path, mode)
}

pub fn mkdir(path: *const c_char) -> isize {
    let default_mode = 0o755; 
        sys_mkdirat(AT_FDCWD, path, default_mode)
}

pub fn pause() -> isize {
    syscall::sys_pause()
}

pub fn rename(old_path: *const c_char, new_path: *const c_char) -> isize {
    syscall::sys_rename(old_path, new_path)
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct Stat {
    pub dev: u64,
    pub ino: u64,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u64,
    pub size: u64,
    pub blksize: u64,
    pub blocks: u64,
    // ... 其他字段
}

impl Stat {
    pub fn new() -> Self {
        Stat::default()
    }
}

pub fn stat(path:*const c_char, stat_buf: &mut Stat) -> isize {
    syscall::sys_stat(path, stat_buf)
}
pub fn fstat(fd: usize, stat_buf: &mut Stat) -> isize {
    syscall::sys_fstat(fd, stat_buf)
}
