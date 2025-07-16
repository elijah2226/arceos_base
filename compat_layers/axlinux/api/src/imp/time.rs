use axerrno::{LinuxError, LinuxResult};
use axhal::time::{TimeValue, monotonic_time, monotonic_time_nanos, nanos_to_ticks, wall_time, ticks_to_nanos};
use linux_raw_sys::general::{
    __kernel_clockid_t, CLOCK_MONOTONIC, CLOCK_REALTIME, timespec, timeval, timezone,
};
use starry_core::task::time_stat_output;

use crate::{ptr::UserPtr, time::TimeValueLike};

pub fn sys_clock_gettime(
    clock_id: __kernel_clockid_t,
    ts: UserPtr<timespec>,
) -> LinuxResult<isize> {
    let now = match clock_id as u32 {
        CLOCK_REALTIME => wall_time(),
        CLOCK_MONOTONIC => monotonic_time(),
        _ => {
            warn!(
                "Called sys_clock_gettime for unsupported clock {}",
                clock_id
            );
            return Err(LinuxError::EINVAL);
        }
    };
    *ts.get_as_mut()? = timespec::from_time_value(now);
    Ok(0)
}

pub fn sys_gettimeofday(ts: UserPtr<timeval>) -> LinuxResult<isize> {
    *ts.get_as_mut()? = timeval::from_time_value(wall_time());
    Ok(0)
}

#[repr(C)]
pub struct Tms {
    /// user time
    tms_utime: usize,
    /// system time
    tms_stime: usize,
    /// user time of children
    tms_cutime: usize,
    /// system time of children
    tms_cstime: usize,
}

pub fn sys_times(tms: UserPtr<Tms>) -> LinuxResult<isize> {
    let (_, utime_us, _, stime_us) = time_stat_output();
    *tms.get_as_mut()? = Tms {
        tms_utime: utime_us,
        tms_stime: stime_us,
        tms_cutime: utime_us,
        tms_cstime: stime_us,
    };
    Ok(nanos_to_ticks(monotonic_time_nanos()) as _)
}

pub fn sys_clock_getres(
    clock_id: __kernel_clockid_t,
    res: UserPtr<timespec>,
) -> LinuxResult<isize> {
    match clock_id as u32 {
        CLOCK_REALTIME | CLOCK_MONOTONIC => {}
        _ => return Err(LinuxError::EINVAL),
    }

    let nanos_per_tick = ticks_to_nanos(1);
    let resolution = TimeValue::from_nanos(nanos_per_tick);

    match res.get_as_mut() {
        Ok(res_ptr) => {
            *res_ptr = timespec::from_time_value(resolution);
        }
        Err(_) => {
            // res is null or invalid, which is fine for this syscall
        }
    }

    Ok(0)
}

// Set the system time
// Abandoned by 64-bit syscall
// Should be called by privileged processes only
// TODO: Implement proper permission checks
/* pub fn sys_stime(new_time: UserPtr<timeval>) -> LinuxResult<isize> {
    let new_time = *new_time.get_as_mut()?;
    let new_time_value = TimeValue::from_secs(new_time.tv_sec as u64)
        + TimeValue::from_nanos(new_time.tv_usec as u64 * 1000);
    
    // Set the system time (this is a placeholder, actual implementation may vary)
    axhal::time::set_wall_time(new_time_value);

    Ok(0)
} */


// Set the system time and timezone
// Should be called by privileged processes only
// TODO: Implement proper permission checks
pub fn sys_settimeofday(
    tv: UserPtr<timeval>, 
    tz: UserPtr<timezone>
) -> LinuxResult<isize> {
    // TODO: Add proper permission checks
    // if !current_process().has_capability(CAP_SYS_TIME) {
    //     return Err(LinuxError::EPERM);
    // }

    // Handle timezone parameter (usually ignored in modern systems)
    if !tz.is_null() {
        // Linux typically ignores the timezone parameter
        // You might want to validate it exists but not use it
        // TODO： get_as_ref()?;
        let _tz = tz.get_as_mut()?;
        // Most implementations just ignore this nowadays
    }

    // Handle the time value
    if !tv.is_null() {
        let time_val = tv.get_as_mut()?;
        
        // Validate the input
        if time_val.tv_usec < 0 || time_val.tv_usec >= 1_000_000 {
            return Err(LinuxError::EINVAL);
        }

        // Convert to the kernel's time representation
        let new_time_value = TimeValue::from_secs(time_val.tv_sec as u64)
            + TimeValue::from_nanos(time_val.tv_usec as u64 * 1000);
        
        // Set the system time
        axhal::time::set_wall_time(new_time_value);
    }

    Ok(0)
}