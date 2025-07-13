use alloc::sync::Arc;
use axerrno::{LinuxError, LinuxResult};
use axfs::{CURRENT_DIR, CURRENT_DIR_PATH};
use axhal::arch::{TrapFrame, UspaceContext};
use axprocess::Pid;
use axsignal::Signo;
use axsync::Mutex;
use axtask::{TaskExtRef, current};
use bitflags::bitflags;
use linux_raw_sys::general::*;
use starry_core::{
    mm::copy_from_kernel,
    task::{ProcessData, TaskExt, ThreadData, add_thread_to_table, new_user_task},
};

use crate::{file::FD_TABLE, ptr::UserPtr};
use axtask::WaitQueue;

bitflags! {
    /// Options for use with [`sys_clone`].
    #[derive(Debug, Clone, Copy, Default)]
    struct CloneFlags: u32 {
        /// The calling process and the child process run in the same
        /// memory space.
        const VM = CLONE_VM;
        /// The caller and the child process share the same  filesystem
        /// information.
        const FS = CLONE_FS;
        /// The calling process and the child process share the same file
        /// descriptor table.
        const FILES = CLONE_FILES;
        /// The calling process and the child process share the same table
        /// of signal handlers.
        const SIGHAND = CLONE_SIGHAND;
        /// If the calling process is being traced, then trace the child
        /// also.
        const PTRACE = CLONE_PTRACE;
        /// The execution of the calling process is suspended until the
        /// child releases its virtual memory resources via a call to
        /// execve(2) or _exit(2) (as with vfork(2)).
        const VFORK = CLONE_VFORK;
        /// The parent of the new child  (as returned by getppid(2))
        /// will be the same as that of the calling process.
        const PARENT = CLONE_PARENT;
        /// The child is placed in the same thread group as the calling
        /// process.
        const THREAD = CLONE_THREAD;
        /// The cloned child is started in a new mount namespace.
        const NEWNS = CLONE_NEWNS;
        /// The child and the calling process share a single list of System
        /// V semaphore adjustment values
        const SYSVSEM = CLONE_SYSVSEM;
        /// The TLS (Thread Local Storage) descriptor is set to tls.
        const SETTLS = CLONE_SETTLS;
        /// Store the child thread ID in the parent's memory.
        const PARENT_SETTID = CLONE_PARENT_SETTID;
        /// Clear (zero) the child thread ID in child memory when the child
        /// exits, and do a wakeup on the futex at that address.
        const CHILD_CLEARTID = CLONE_CHILD_CLEARTID;
        /// A tracing process cannot force `CLONE_PTRACE` on this child
        /// process.
        const UNTRACED = CLONE_UNTRACED;
        /// Store the child thread ID in the child's memory.
        const CHILD_SETTID = CLONE_CHILD_SETTID;
        /// Create the process in a new cgroup namespace.
        const NEWCGROUP = CLONE_NEWCGROUP;
        /// Create the process in a new UTS namespace.
        const NEWUTS = CLONE_NEWUTS;
        /// Create the process in a new IPC namespace.
        const NEWIPC = CLONE_NEWIPC;
        /// Create the process in a new user namespace.
        const NEWUSER = CLONE_NEWUSER;
        /// Create the process in a new PID namespace.
        const NEWPID = CLONE_NEWPID;
        /// Create the process in a new network namespace.
        const NEWNET = CLONE_NEWNET;
        /// The new process shares an I/O context with the calling process.
        const IO = CLONE_IO;
    }
}

pub fn sys_clone(
    tf: &TrapFrame,
    flags: u32,
    stack: usize,
    parent_tid: usize,
    #[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))] child_tid: usize,
    tls: usize,
    #[cfg(not(any(target_arch = "x86_64", target_arch = "loongarch64")))] child_tid: usize,
) -> LinuxResult<isize> {
    const FLAG_MASK: u32 = 0xff;
    let exit_signal = flags & FLAG_MASK;
    let flags = CloneFlags::from_bits_truncate(flags & !FLAG_MASK);

    info!(
        "sys_clone <= flags: {:?}, exit_signal: {}, stack: {:#x}, ptid: {:#x}, ctid: {:#x}, tls: {:#x}",
        flags, exit_signal, stack, parent_tid, child_tid, tls
    );

    if exit_signal != 0 && flags.contains(CloneFlags::THREAD | CloneFlags::PARENT) {
        return Err(LinuxError::EINVAL);
    }
    if flags.contains(CloneFlags::THREAD) && !flags.contains(CloneFlags::VM | CloneFlags::SIGHAND) {
        return Err(LinuxError::EINVAL);
    }
    let exit_signal = Signo::from_repr(exit_signal as u8);

    let mut new_uctx = UspaceContext::from(tf);
    if stack != 0 {
        new_uctx.set_sp(stack);
    }
    if flags.contains(CloneFlags::SETTLS) {
        new_uctx.set_tls(tls);
    }
    new_uctx.set_retval(0);

    let set_child_tid = if flags.contains(CloneFlags::CHILD_SETTID) {
        Some(UserPtr::<u32>::from(child_tid).get_as_mut()?)
    } else {
        None
    };

    let curr = current();
    let mut new_task = new_user_task(curr.name(), new_uctx, set_child_tid);

    let tid = new_task.id().as_u64() as Pid;
    if flags.contains(CloneFlags::PARENT_SETTID) {
        *UserPtr::<Pid>::from(parent_tid).get_as_mut()? = tid;
    }

    // --- 新增 vfork 处理逻辑 ---
    // 1. 如果是 vfork，创建一个完成端口
    let vfork_wait_queue = if flags.contains(CloneFlags::VFORK) {
        // 创建一个新的等待队列
        Some(Arc::new(WaitQueue::new()))
    } else {
        None
    };
    // --- 新增结束 ---

    let process = if flags.contains(CloneFlags::THREAD) {
        new_task.ctx_mut().set_page_table_root(
            curr.task_ext()
                .process_data()
                .aspace
                .lock()
                .page_table_root(),
        );

        curr.task_ext().thread.process()
    } else {
        let parent = if flags.contains(CloneFlags::PARENT) {
            curr.task_ext()
                .thread
                .process()
                .parent()
                .ok_or(LinuxError::EINVAL)?
        } else {
            curr.task_ext().thread.process().clone()
        };
        let builder = parent.fork(tid);

        let aspace = if flags.contains(CloneFlags::VM) {
            curr.task_ext().process_data().aspace.clone()
        } else {
            let mut aspace = curr.task_ext().process_data().aspace.lock();
            let mut aspace = aspace.try_clone()?;
            copy_from_kernel(&mut aspace)?;
            Arc::new(Mutex::new(aspace))
        };
        new_task
            .ctx_mut()
            .set_page_table_root(aspace.lock().page_table_root());

        let signal_actions = if flags.contains(CloneFlags::SIGHAND) {
            parent
                .data::<ProcessData>()
                .map_or_else(Arc::default, |it| it.signal.actions.clone())
        } else {
            Arc::default()
        };

         // 1. 从 parent 对象中安全地获取父进程的 ProcessData
        let parent_data = parent
            .data::<ProcessData>()
            .ok_or(LinuxError::EPERM)?; // 如果获取失败，说明有问题，返回权限错误

        // 2. 调用我们新创建的 fork_from 构造函数
        let process_data = ProcessData::fork_from(
            parent_data,
            aspace, // a. 传递为子进程准备好的地址空间
            signal_actions, // b. 传递为子进程准备好的信号处理器
            exit_signal, // c. 传递子进程的退出信号
        );

        // 如果是 vfork，将完成端口的另一半存入子进程的 ProcessData
        if let Some(wq) = &vfork_wait_queue {
            *process_data.vfork_completion.lock() = Some(wq.clone());
        }

        if flags.contains(CloneFlags::FILES) {
            FD_TABLE
                .deref_from(&process_data.ns)
                .init_shared(FD_TABLE.share());
        } else {
            FD_TABLE
                .deref_from(&process_data.ns)
                .init_new(FD_TABLE.copy_inner());
        }

        if flags.contains(CloneFlags::FS) {
            CURRENT_DIR
                .deref_from(&process_data.ns)
                .init_shared(CURRENT_DIR.share());
            CURRENT_DIR_PATH
                .deref_from(&process_data.ns)
                .init_shared(CURRENT_DIR_PATH.share());
        } else {
            CURRENT_DIR
                .deref_from(&process_data.ns)
                .init_new(CURRENT_DIR.copy_inner());
            CURRENT_DIR_PATH
                .deref_from(&process_data.ns)
                .init_new(CURRENT_DIR_PATH.copy_inner());
        }
        &builder.data(process_data).build()
    };

    let thread_data = ThreadData::new(process.data().unwrap());
    if flags.contains(CloneFlags::CHILD_CLEARTID) {
        thread_data.set_clear_child_tid(child_tid);
    }

    let thread = process.new_thread(tid).data(thread_data).build();
    add_thread_to_table(&thread);
    new_task.init_task_ext(TaskExt::new(thread));
    axtask::spawn_task(new_task);
    // --- 新增 vfork 父进程等待逻辑 ---
    // 6. 如果是 vfork，父进程在此等待
    if let Some(wq) = vfork_wait_queue {
        info!("vfork: parent {:?} is waiting for child...", curr.id());
        wq.wait();
        info!("vfork: parent {:?} is woken up.", curr.id());
    }
    // --- 新增结束 ---

    Ok(tid as _)
}

pub fn sys_fork(tf: &TrapFrame) -> LinuxResult<isize> {
    sys_clone(tf, SIGCHLD, 0, 0, 0, 0)
}

/// sys_vfork is equivalent to clone(CLONE_VFORK | CLONE_VM | SIGCHLD, 0, ...);
pub fn sys_vfork(tf: &TrapFrame) -> LinuxResult<isize> {
    // CLONE_VM 意味着共享地址空间
    // CLONE_VFORK 是 vfork 的特殊标志
    // SIGCHLD 是子进程退出时发送给父进程的信号
    const VFORK_FLAGS: u32 = CLONE_VFORK | CLONE_VM | SIGCHLD;
    
    // 直接调用 sys_clone，传递 vfork 特定的标志位
    sys_clone(tf, VFORK_FLAGS, 0, 0, 0, 0)
}
