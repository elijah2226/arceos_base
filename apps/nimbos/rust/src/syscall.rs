use super::time::{ClockId, TimeSpec};
use crate::arch::syscall;

pub use crate::arch::sys_clone;
use crate::Stat;
use core::ffi::c_char;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        pub const SYSCALL_READ: usize = 0;
        pub const SYSCALL_WRITE: usize = 1;
        pub const SYSCALL_STAT: usize = 4; 
        pub const SYSCALL_YIELD: usize = 24;
        pub const SYSCALL_GETPID: usize = 39;
        pub const SYSCALL_CLONE: usize = 56;
        pub const SYSCALL_FORK: usize = 57;
        pub const SYSCALL_VFORK: usize = 58;  
        pub const SYSCALL_EXEC: usize = 59;
        pub const SYSCALL_EXIT: usize = 60;
        pub const SYSCALL_WAITPID: usize = 61;
        pub const SYSCALL_CLOCK_GETTIME: usize = 228;
        pub const SYSCALL_CLOCK_NANOSLEEP: usize = 230;
        pub const SYSCALL_SYMLINK: usize = 88; 
        pub const SYSCALL_READLINK: usize = 89; 
        pub const SYSCALL_CHMOD: usize = 90;    
        pub const SYSCALL_CHOWN: usize = 92; 
        pub const SYSCALL_OPEN: usize = 2;    
        pub const SYSCALL_CLOSE: usize = 3;   
        pub const SYSCALL_UNLINK: usize = 87; 
        pub const SYSCALL_RMDIR: usize = 84;     
        pub const SYSCALL_FSTAT: usize = 5; 
        pub const SYSCALL_ACCESS: usize = 21; 
        pub const SYSCALL_MKNODAT: usize = 259; 
        pub const SYSCALL_MKDIRAT: usize = 258;
        pub const SYSCALL_PAUSE: usize = 34; 
        pub const SYSCALL_RENAME: usize = 82;
        pub const SYSCALL_SET_TIMEOFDAY: usize = 164;
        pub const SYSCALL_GET_TIMEOFDAY: usize = 96;
        pub const SYSCALL_CLOCK_GETRES: usize = 229;   
    }
    else {
        pub const SYSCALL_READ: usize = 63;
        pub const SYSCALL_WRITE: usize = 64;
        pub const SYSCALL_YIELD: usize = 124;
        pub const SYSCALL_GETPID: usize = 172;
        #[allow(dead_code)]
        pub const SYSCALL_CLONE: usize = 220;
        pub const SYSCALL_FORK: usize = 220;
        pub const SYSCALL_VFORK: usize = 221;
        pub const SYSCALL_EXEC: usize = 221;
        pub const SYSCALL_EXIT: usize = 93;
        pub const SYSCALL_WAITPID: usize = 260;
        pub const SYSCALL_CLOCK_GETTIME: usize = 403;
        pub const SYSCALL_CLOCK_NANOSLEEP: usize = 407;
        pub const SYSCALL_SET_TIMEOFDAY: usize = 170;
        pub const SYSCALL_GET_TIMEOFDAY: usize = 169;
        pub const SYSCALL_CLOCK_GETRES: usize = 114;
    }
}

pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(SYSCALL_READ, [
        fd,
        buffer.as_mut_ptr() as usize,
        buffer.len(),
    ])
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys_exit never returns!");
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

pub fn sys_vfork() -> isize {
    syscall(SYSCALL_VFORK, [0, 0, 0])
}

pub fn sys_exec(path: &str) -> isize {
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, 0, 0])
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32, options: u32) -> isize {
    syscall(SYSCALL_WAITPID, [
        pid as usize,
        exit_code as _,
        options as _,
    ])
}

pub fn sys_clock_gettime(clk: ClockId, req: &mut TimeSpec) -> isize {
    syscall(SYSCALL_CLOCK_GETTIME, [clk as _, req as *mut _ as usize, 0])
}

pub fn sys_clock_nanosleep(clk: ClockId, flags: u32, req: &TimeSpec) -> isize {
    syscall(SYSCALL_CLOCK_NANOSLEEP, [
        clk as _,
        flags as _,
        req as *const _ as usize,
    ])
}

pub fn sys_symlink(target: *const c_char, linkpath: *const c_char) -> isize {
    syscall(SYSCALL_SYMLINK, [target as usize, linkpath as usize, 0])
}

pub fn sys_readlink(path: *const c_char, buf: &mut [u8]) -> isize {
    syscall(SYSCALL_READLINK, [path as usize, buf.as_mut_ptr() as usize, buf.len()])
}

pub fn sys_chmod(path: *const c_char, mode: u32) -> isize {
    syscall(SYSCALL_CHMOD, [path as usize, mode as usize, 0])
}

pub fn sys_chown(path: *const c_char, uid: u32, gid: u32) -> isize {
    syscall(SYSCALL_CHOWN, [path as usize, uid as usize, gid as usize])
}

pub fn sys_open(path: *const c_char, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path as usize, flags as usize, 0])
}

pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd , 0, 0])
}

pub fn sys_unlink(path: *const c_char) -> isize {
    syscall(SYSCALL_UNLINK, [path as usize, 0, 0])
}

pub fn sys_rmdir(path: *const c_char) -> isize {
    syscall(SYSCALL_RMDIR, [path as usize, 0, 0])
}

pub fn sys_stat(path: *const c_char, stat_buf: &mut Stat) -> isize {
    syscall(SYSCALL_STAT, [path as usize, stat_buf as *mut _ as usize, 0])
}

pub fn sys_fstat(fd: usize, stat_buf: &mut Stat) -> isize {
    syscall(SYSCALL_FSTAT, [fd, stat_buf as *mut _ as usize, 0])
}

pub fn sys_access(path: *const c_char, mode: u32) -> isize {
    syscall(SYSCALL_ACCESS, [path as usize, mode as usize, 0])
}

pub fn sys_mknodat(dirfd: i32, path: *const c_char, mode: u32) -> isize {
    syscall(SYSCALL_MKNODAT, [dirfd as usize, path as usize, mode as usize])
}

pub fn sys_mkdirat(dirfd: i32, path: *const c_char, mode: u32) -> isize {
    syscall(SYSCALL_MKDIRAT, [dirfd as usize, path as usize, mode as usize])
}

pub fn sys_pause() -> isize {
    syscall(SYSCALL_PAUSE, [0, 0, 0])
}

pub fn sys_rename(old_path: *const c_char, new_path: *const c_char) -> isize {
    syscall(SYSCALL_RENAME, [old_path as usize, new_path as usize, 0])
}

pub fn sys_settimeofday(tv: &TimeSpec, tz: *const ()) -> isize {
    syscall(SYSCALL_SET_TIMEOFDAY, [tv as *const _ as usize, tz as usize, 0])
}

pub fn sys_gettimeofday(tv: *mut TimeSpec, tz: *mut ()) -> isize {
    syscall(SYSCALL_GET_TIMEOFDAY, [tv as usize, tz as usize, 0])
}

pub fn sys_clock_getres(clk: ClockId, res: &mut TimeSpec) -> isize {
    syscall(SYSCALL_CLOCK_GETTIME, [clk as _, res as *mut _ as usize, 0])
}