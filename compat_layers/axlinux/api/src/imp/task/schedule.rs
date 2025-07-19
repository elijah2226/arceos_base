use axerrno::{LinuxError, LinuxResult};
use linux_raw_sys::general::{timespec,CLOCK_MONOTONIC,CLOCK_REALTIME,TIMER_ABSTIME,__kernel_clockid_t};

use crate::{
    ptr::{UserConstPtr, UserPtr, nullable},
    time::TimeValueLike,
};

use axtask::current;
// use axsignal::Signo;

pub fn sys_sched_yield() -> LinuxResult<isize> {
    axtask::yield_now();
    Ok(0)
}

/// Sleep some nanoseconds
///
/// TODO: should be woken by signals, and set errno
pub fn sys_nanosleep(req: UserConstPtr<timespec>, rem: UserPtr<timespec>) -> LinuxResult<isize> {
    let req = req.get_as_ref()?;

    if req.tv_nsec < 0 || req.tv_nsec > 999_999_999 || req.tv_sec < 0 {
        return Err(LinuxError::EINVAL);
    }

    let dur = req.to_time_value();
    debug!("sys_nanosleep <= {:?}", dur);

    let now = axhal::time::monotonic_time();

    axtask::sleep(dur);

    let after = axhal::time::monotonic_time();
    let actual = after - now;

    if let Some(diff) = dur.checked_sub(actual) {
        if let Some(rem) = nullable!(rem.get_as_mut())? {
            *rem = timespec::from_time_value(diff);
        }
        Err(LinuxError::EINTR)
    } else {
        Ok(0)
    }
}


// Sleep some nanoseconds wi/wo absolute time
// TODO: should be woken by signals, and set errno
pub fn sys_clock_nanosleep(
    clock_id: __kernel_clockid_t,
    flags: u32,
    req: UserConstPtr<timespec>,
    rem: UserPtr<timespec>,
) -> LinuxResult<isize> {
    let req = req.get_as_ref()?;

    if req.tv_nsec < 0 || req.tv_nsec > 999_999_999 || req.tv_sec < 0 {
        return Err(LinuxError::EINVAL);
    }

    let req_dur = req.to_time_value();

    debug!(
        "sys_clock_nanosleep <= clock_id: {}, flags: {}, req: {:?}",
        clock_id, flags, req_dur
    );

    // 只支持 CLOCK_MONOTONIC & CLOCK_REALTIME
    let now = match clock_id as u32 {
        CLOCK_MONOTONIC => axhal::time::monotonic_time(),
        CLOCK_REALTIME => axhal::time::wall_time(),
        _ => return Err(LinuxError::EINVAL),
    };

    let sleep_dur = if flags & TIMER_ABSTIME as u32 != 0 {
        // TIMER_ABSTIME：绝对时间
        if req_dur <= now {
            return Ok(0); // 已经过期，不睡
        }
        req_dur - now
    } else {
        // 相对时间
        req_dur
    };

    let start = now;

    axtask::sleep(sleep_dur);

    let end = match clock_id as u32 {
        CLOCK_MONOTONIC => axhal::time::monotonic_time(),
        CLOCK_REALTIME => axhal::time::wall_time(),
        _ => return Err(LinuxError::EINVAL),
    };
    let actual = end - start;

    // 如果被信号打断（第一版可以先不实现信号，直接按 nanosleep 写法）
    if let Some(diff) = sleep_dur.checked_sub(actual) {
        if let Some(rem) = nullable!(rem.get_as_mut())? {
            *rem = timespec::from_time_value(diff);
        }
        Err(LinuxError::EINTR)
    } else {
        Ok(0)
    }
}

pub fn sys_pause() -> LinuxResult<isize> {
    info!("sys_pause <= Task({:?}) pausing...", current().id());

    // axtask 应该有一个让任务永久休眠的方法，或者使用 WaitQueue。
    // 我们用一个永远不会被 notify 的 WaitQueue 模拟：
    use axtask::WaitQueue; // 确保 WaitQueue 导入

    let dummy_wq = WaitQueue::new();
    dummy_wq.wait(); // 当前任务将永久休眠在此，直到有信号将其唤醒

    Err(LinuxError::EINTR)
}
