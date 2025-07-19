use axerrno::LinuxError;
use core::ffi::c_int;
use linux_raw_sys::general::{rlimit,RLIMIT_STACK,RLIMIT_DATA,RLIMIT_NOFILE};
use crate::file::AX_FILE_LIMIT;

/// Get resource limitations
///
/// TODO: support more resource types
pub fn sys_getrlimit(resource: c_int, rlimits: *mut rlimit) -> Result<c_int, LinuxError> {
    if rlimits.is_null() {
        return Ok(0);
    }
    match resource as u32 {
        RLIMIT_STACK => unsafe{
            (*rlimits).rlim_cur = axconfig::TASK_STACK_SIZE as _;
            (*rlimits).rlim_max = axconfig::TASK_STACK_SIZE as _;
        },
        RLIMIT_NOFILE => unsafe {
            (*rlimits).rlim_cur = AX_FILE_LIMIT as _;
            (*rlimits).rlim_max = AX_FILE_LIMIT as _;
        },
        _ => {}
    }
    Ok(0)
}


/// Set resource limitations
///
/// TODO: support more resource types
pub fn sys_setrlimit(resource: c_int, rlimits: *mut rlimit) -> Result<c_int, LinuxError> {
    match resource as u32 {
        RLIMIT_DATA => {}
        RLIMIT_STACK => {}
        RLIMIT_NOFILE => {}
        _ => return Err(LinuxError::EINVAL),
    }
    // Currently do not support set resources
    Ok(0)
}
