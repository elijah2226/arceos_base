use core::{
    ffi::{c_char, c_int, c_void},
    mem::offset_of,
};

use alloc::ffi::CString;
use axerrno::{LinuxError, LinuxResult};
use axfs::fops::DirEntry;
use linux_raw_sys::general::{
    AT_FDCWD, AT_REMOVEDIR, DT_BLK, DT_CHR, DT_DIR, 
    DT_FIFO, DT_LNK, DT_REG, DT_SOCK, DT_UNKNOWN, 
    linux_dirent64,
};

use crate::{
    file::{Directory, FileLike},
    path::{HARDLINK_MANAGER, handle_file_path},
    ptr::{UserConstPtr, UserPtr, nullable},
};

use axfs::fops::FilePerm;
use axfs::fops::FileType as AxFileType;
use axfs::api;
use linux_raw_sys::general::{
        S_IFMT, S_IFREG, S_IFDIR, S_IFLNK, S_IFIFO, S_IFCHR, S_IFBLK
    };
// use axtask::{TaskExtRef, current};

/// The ioctl() system call manipulates the underlying device parameters
/// of special files.
///
/// # Arguments
/// * `fd` - The file descriptor
/// * `op` - The request code. It is of type unsigned long in glibc and BSD,
///   and of type int in musl and other UNIX systems.
/// * `argp` - The argument to the request. It is a pointer to a memory location
pub fn sys_ioctl(_fd: i32, _op: usize, _argp: UserPtr<c_void>) -> LinuxResult<isize> {
    warn!("Unimplemented syscall: SYS_IOCTL");
    Ok(0)
}

pub fn sys_chdir(path: UserConstPtr<c_char>) -> LinuxResult<isize> {
    let path = path.get_as_str()?;
    debug!("sys_chdir <= {:?}", path);

    axfs::api::set_current_dir(path)?;
    Ok(0)
}

pub fn sys_mkdirat(dirfd: i32, path: UserConstPtr<c_char>, mode: u32) -> LinuxResult<isize> {
    let path = path.get_as_str()?;
    debug!(
        "sys_mkdirat <= dirfd: {}, path: {}, mode: {}",
        dirfd, path, mode
    );

    if mode != 0 {
        warn!("directory mode not supported.");
    }

    let path = handle_file_path(dirfd, path)?;
    axfs::api::create_dir(path.as_str())?;

    Ok(0)
}

pub fn sys_rmdir(path: UserConstPtr<c_char>) -> LinuxResult<isize> {
    sys_unlinkat(AT_FDCWD, path, AT_REMOVEDIR)
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileType {
    Unknown = DT_UNKNOWN as u8,
    Fifo = DT_FIFO as u8,
    Chr = DT_CHR as u8,
    Dir = DT_DIR as u8,
    Blk = DT_BLK as u8,
    Reg = DT_REG as u8,
    Lnk = DT_LNK as u8,
    Socket = DT_SOCK as u8,
}

impl From<axfs::api::FileType> for FileType {
    fn from(ft: axfs::api::FileType) -> Self {
        match ft {
            ft if ft.is_dir() => FileType::Dir,
            ft if ft.is_file() => FileType::Reg,
            _ => FileType::Unknown,
        }
    }
}

// Directory buffer for getdents64 syscall
struct DirBuffer<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> DirBuffer<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, offset: 0 }
    }

    fn remaining_space(&self) -> usize {
        self.buf.len().saturating_sub(self.offset)
    }

    fn write_entry(&mut self, d_type: FileType, name: &[u8]) -> bool {
        const NAME_OFFSET: usize = offset_of!(linux_dirent64, d_name);

        let len = NAME_OFFSET + name.len() + 1;
        // alignment
        let len = len.next_multiple_of(align_of::<linux_dirent64>());
        if self.remaining_space() < len {
            return false;
        }

        unsafe {
            let entry_ptr = self.buf.as_mut_ptr().add(self.offset);
            entry_ptr.cast::<linux_dirent64>().write(linux_dirent64 {
                // FIXME: real inode number
                d_ino: 1,
                d_off: 0,
                d_reclen: len as _,
                d_type: d_type as _,
                d_name: Default::default(),
            });

            let name_ptr = entry_ptr.add(NAME_OFFSET);
            name_ptr.copy_from_nonoverlapping(name.as_ptr(), name.len());
            name_ptr.add(name.len()).write(0);
        }

        self.offset += len;
        true
    }
}

pub fn sys_getdents64(fd: i32, buf: UserPtr<u8>, len: usize) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_slice(len)?;
    debug!(
        "sys_getdents64 <= fd: {}, buf: {:p}, len: {}",
        fd,
        buf.as_ptr(),
        buf.len()
    );

    let mut buffer = DirBuffer::new(buf);

    let dir = Directory::from_fd(fd)?;

    let mut last_dirent = dir.last_dirent();
    if let Some(ent) = last_dirent.take()
        && !buffer.write_entry(ent.entry_type().into(), ent.name_as_bytes())
    {
        *last_dirent = Some(ent);
        return Err(LinuxError::EINVAL);
    }

    let mut inner = dir.inner();
    loop {
        let mut dirents = [DirEntry::default()];
        let cnt = inner.read_dir(&mut dirents)?;
        if cnt == 0 {
            break;
        }

        let [ent] = dirents;
        if !buffer.write_entry(ent.entry_type().into(), ent.name_as_bytes()) {
            *last_dirent = Some(ent);
            break;
        }
    }

    if last_dirent.is_some() && buffer.offset == 0 {
        return Err(LinuxError::EINVAL);
    }
    Ok(buffer.offset as _)
}

/// create a link from new_path to old_path
/// old_path: old file path
/// new_path: new file path
/// flags: link flags
/// return value: return 0 when success, else return -1.
pub fn sys_linkat(
    old_dirfd: c_int,
    old_path: UserConstPtr<c_char>,
    new_dirfd: c_int,
    new_path: UserConstPtr<c_char>,
    flags: i32,
) -> LinuxResult<isize> {
    let old_path = old_path.get_as_str()?;
    let new_path = new_path.get_as_str()?;
    debug!(
        "sys_linkat <= old_dirfd: {}, old_path: {}, new_dirfd: {}, new_path: {}, flags: {}",
        old_dirfd, old_path, new_dirfd, new_path, flags
    );

    if flags != 0 {
        warn!("Unsupported flags: {flags}");
    }

    // handle old path
    let old_path = handle_file_path(old_dirfd, old_path)?;
    // handle new path
    let new_path = handle_file_path(new_dirfd, new_path)?;

    HARDLINK_MANAGER.create_link(&new_path, &old_path)?;

    Ok(0)
}

pub fn sys_link(
    old_path: UserConstPtr<c_char>,
    new_path: UserConstPtr<c_char>,
) -> LinuxResult<isize> {
    sys_linkat(AT_FDCWD, old_path, AT_FDCWD, new_path, 0)
}

/// remove link of specific file (can be used to delete file)
/// dir_fd: the directory of link to be removed
/// path: the name of link to be removed
/// flags: can be 0 or AT_REMOVEDIR
/// return 0 when success, else return -1
pub fn sys_unlinkat(dirfd: c_int, path: UserConstPtr<c_char>, flags: u32) -> LinuxResult<isize> {
    let path = path.get_as_str()?;
    debug!(
        "sys_unlinkat <= dirfd: {}, path: {}, flags: {}",
        dirfd, path, flags
    );

    let path = handle_file_path(dirfd, path)?;

    if flags == AT_REMOVEDIR {
        axfs::api::remove_dir(path.as_str())?;
    } else {
        let metadata = axfs::api::metadata(path.as_str())?;
        if metadata.is_dir() {
            return Err(LinuxError::EISDIR);
        } else {
            debug!("unlink file: {:?}", path);
            HARDLINK_MANAGER
                .remove_link(&path)
                .ok_or(LinuxError::ENOENT)?;
        }
    }
    Ok(0)
}

pub fn sys_unlink(path: UserConstPtr<c_char>) -> LinuxResult<isize> {
    sys_unlinkat(AT_FDCWD, path, 0)
}

pub fn sys_getcwd(buf: UserPtr<u8>, size: usize) -> LinuxResult<isize> {
    let buf = nullable!(buf.get_as_mut_slice(size))?;

    let Some(buf) = buf else {
        return Ok(0);
    };

    let cwd = CString::new(axfs::api::current_dir()?).map_err(|_| LinuxError::EINVAL)?;
    let cwd = cwd.as_bytes_with_nul();

    if cwd.len() <= buf.len() {
        buf[..cwd.len()].copy_from_slice(cwd);
        Ok(buf.as_ptr() as _)
    } else {
        Err(LinuxError::ERANGE)
    }
}

// Symlink
pub fn sys_symlink(target: UserConstPtr<c_char>, linkpath: UserConstPtr<c_char>) -> LinuxResult<isize> {
    let target_str = target.get_as_str()?;
    let linkpath_str = linkpath.get_as_str()?;
    axfs::api::create_symlink(target_str, linkpath_str)?;
    Ok(0)
}

// Readlink
pub fn sys_readlink(path: UserConstPtr<c_char>, buf: UserPtr<u8>, bufsiz: usize) -> LinuxResult<isize> {
    let user_buf = buf.get_as_mut_slice(bufsiz)?;
    let path_str = path.get_as_str()?;
    let target_path = axfs::api::read_link(path_str)?;
    let target_bytes = target_path.as_bytes();
    let copy_len = user_buf.len().min(target_bytes.len());
    user_buf[..copy_len].copy_from_slice(&target_bytes[..copy_len]);
    Ok(copy_len as isize)
}

// Chmod
pub fn sys_chmod(path: UserConstPtr<c_char>, mode: u32) -> LinuxResult<isize> {
    let path_str = path.get_as_str()?;
    // FilePerm 包含了权限位，直接使用它
    let perm = FilePerm::from_bits_truncate(mode as u16);
    axfs::api::set_permission(path_str, perm)?;
    Ok(0)
}

// Chown
pub fn sys_chown(path: UserConstPtr<c_char>, owner: u32, group: u32) -> LinuxResult<isize> {
    let path_str = path.get_as_str()?;
    axfs::api::set_owner(path_str, owner, group)?;
    Ok(0)
}


pub fn sys_faccessat(dirfd: c_int, path: UserConstPtr<c_char>, mode: u32, _flags: u32) -> LinuxResult<isize> {
    // TODO: 处理 AT_EACCESS flag，它要求使用有效ID(euid, egid)而不是真实ID来检查
    let path = handle_file_path(dirfd, path.get_as_str()?)?;
    axfs::api::access(path.as_str(), mode)?;
    Ok(0)
}

pub fn sys_access(path: UserConstPtr<c_char>, mode: u32) -> LinuxResult<isize> {
    sys_faccessat(AT_FDCWD, path, mode, 0)
}

pub fn sys_mknodat(dirfd: c_int, path: UserConstPtr<c_char>, mode: u32, _dev: u64) -> LinuxResult<isize> {
    // mode 的高位是文件类型，低位是权限
    let file_type = match mode & S_IFMT {
        S_IFREG => AxFileType::File,
        S_IFDIR => AxFileType::Dir,
        S_IFLNK => AxFileType::SymLink,
        S_IFIFO => AxFileType::Fifo,
        S_IFCHR => AxFileType::CharDevice,
        S_IFBLK => AxFileType::BlockDevice,
        _ => return Err(LinuxError::EINVAL),
    };
    let perm = FilePerm::from_bits_truncate(mode as u16);
    
    let path = handle_file_path(dirfd, path.get_as_str()?)?;
    axfs::api::mknod(path.as_str(), file_type, perm)?;
    // TODO: 对于设备文件，需要用 dev 来注册设备
    Ok(0)
}

pub fn sys_rename(old_path_ptr: UserConstPtr<c_char>, new_path_ptr: UserConstPtr<c_char>) -> LinuxResult<isize> {
    let old_path = old_path_ptr.get_as_str()?;
    let new_path = new_path_ptr.get_as_str()?;
    debug!("sys_rename <= old: {:?}, new: {:?}", old_path, new_path);

    api::rename(old_path, new_path)?;

    Ok(0) 
}