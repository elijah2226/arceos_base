use core::ffi::c_char;

use axerrno::LinuxResult;
use linux_raw_sys::system::new_utsname;

use crate::ptr::UserPtr;
use axtask::current;
use axtask::TaskExtRef;

pub fn sys_getuid() -> LinuxResult<isize> {
     // 1. 获取当前任务
    let task = current();
    // 2. 通过 TaskExt 访问其所属进程的数据
    let cred = task.task_ext().process_data().cred.lock();
    // 3. 返回真实的 UID
    Ok(cred.uid as isize)
}

pub fn sys_geteuid() -> LinuxResult<isize> {

    let task = current();
    let cred = task.task_ext().process_data().cred.lock();
    // 返回有效的 UID
    Ok(cred.euid as isize)
}

pub fn sys_getgid() -> LinuxResult<isize> {

    let task = current();
    let cred = task.task_ext().process_data().cred.lock();
    // 返回有效的 UID
    Ok(cred.euid as isize)
}

pub fn sys_getegid() -> LinuxResult<isize> {

    let task = current();
    let cred = task.task_ext().process_data().cred.lock();
    // 返回有效的 GID
    Ok(cred.egid as isize)
}

const fn pad_str(info: &str) -> [c_char; 65] {
    let mut data: [c_char; 65] = [0; 65];
    // this needs #![feature(const_copy_from_slice)]
    // data[..info.len()].copy_from_slice(info.as_bytes());
    unsafe {
        core::ptr::copy_nonoverlapping(info.as_ptr().cast(), data.as_mut_ptr(), info.len());
    }
    data
}

const UTSNAME: new_utsname = new_utsname {
    sysname: pad_str("Starry"),
    nodename: pad_str("Starry - machine[0]"),
    release: pad_str("10.0.0"),
    version: pad_str("10.0.0"),
    machine: pad_str("10.0.0"),
    domainname: pad_str("https://github.com/oscomp/starry-next"),
};

pub fn sys_uname(name: UserPtr<new_utsname>) -> LinuxResult<isize> {
    *name.get_as_mut()? = UTSNAME;
    Ok(0)
}