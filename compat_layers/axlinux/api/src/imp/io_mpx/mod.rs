//! I/O multiplexing:
//!
//! * [`select`](select::sys_select)
//! * [`epoll_create`](epoll::sys_epoll_create)
//! * [`epoll_ctl`](epoll::sys_epoll_ctl)
//! * [`epoll_wait`](epoll::sys_epoll_wait)

mod epoll;
mod select;
pub use self::epoll::{sys_epoll_create, sys_epoll_ctl, sys_epoll_wait};
pub use self::select::sys_select;
