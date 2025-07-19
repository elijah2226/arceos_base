//! [`std::fs`]-like high-level filesystem manipulation operations.
#![allow(unused)]
mod dir;
mod file;

pub use self::dir::{DirBuilder, DirEntry, ReadDir};
pub use self::file::{File, FileType, Metadata, OpenOptions, Permissions};

use alloc::{string::String, vec::Vec};
use axio::{self as io, prelude::*};
use axio::Result as IoResult;

use axerrno::{ax_err, AxResult, AxError};
use crate::dev::Disk;
// 条件编译导入
#[cfg(feature = "lwext4_rs")]
use crate::fs::lwext4_rust::FileWrapper as Ext4FileWrapper;
#[cfg(feature = "fatfs")]
use crate::fs::fatfs::{FileWrapper as FatFileWrapper, DirWrapper as FatDirWrapper};

/// Returns an iterator over the entries within a directory.
pub fn read_dir(path: &str) -> io::Result<ReadDir> {
    ReadDir::new(path)
}

/// Returns the canonical, absolute form of a path with all intermediate
/// components normalized.
pub fn canonicalize(path: &str) -> io::Result<String> {
    crate::root::absolute_path(path)
}

/// Returns the current working directory as a [`String`].
pub fn current_dir() -> io::Result<String> {
    crate::root::current_dir()
}

/// Changes the current working directory to the specified path.
pub fn set_current_dir(path: &str) -> io::Result<()> {
    crate::root::set_current_dir(path)
}

/// Read the entire contents of a file into a bytes vector.
pub fn read(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(size as usize);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Read the entire contents of a file into a string.
pub fn read_to_string(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut string = String::with_capacity(size as usize);
    file.read_to_string(&mut string)?;
    Ok(string)
}

/// Write a slice as the entire contents of a file.
pub fn write<C: AsRef<[u8]>>(path: &str, contents: C) -> io::Result<()> {
    File::create(path)?.write_all(contents.as_ref())
}

/// Given a path, query the file system to get information about a file,
/// directory, etc.
pub fn metadata(path: &str) -> io::Result<Metadata> {
    crate::root::lookup(None, path)?.get_attr().map(Metadata)
}

/// Creates a new, empty directory at the provided path.
pub fn create_dir(path: &str) -> io::Result<()> {
    DirBuilder::new().create(path)
}

/// Recursively create a directory and all of its parent components if they
/// are missing.
pub fn create_dir_all(path: &str) -> io::Result<()> {
    DirBuilder::new().recursive(true).create(path)
}

/// Removes an empty directory.
pub fn remove_dir(path: &str) -> io::Result<()> {
    crate::root::remove_dir(None, path)
}

/// Removes a file from the filesystem.
pub fn remove_file(path: &str) -> io::Result<()> {
    crate::root::remove_file(None, path)
}

/// Rename a file or directory to a new name.
/// Delete the original file if `old` already exists.
///
/// This only works then the new path is in the same mounted fs.
pub fn rename(old: &str, new: &str) -> io::Result<()> {
    crate::root::rename(old, new)
}

/// check whether absolute path exists.
pub fn absolute_path_exists(path: &str) -> bool {
    crate::root::lookup(None, path).is_ok()
}

/// Checks the access permissions of a file or directory at the given path.
pub fn access(path: &str, mode: u32) -> io::Result<()> {
    crate::root::access(None, path, mode)
}

/// Creates a filesystem node (file, device, etc.) at the specified path.
pub fn mknod(path: &str, ty: FileType, perm: Permissions) -> io::Result<()> {
    let (parent, name) = crate::root::lookup_parent(None, path)?;
    parent.create(name, ty)?;
    #[cfg(feature = "lwext4_rs")]
    if let Some(ext_dir_wrapper) = parent.as_any().downcast_ref::<crate::fs::lwext4_rust::FileWrapper>() {
        ext_dir_wrapper.set_permission(perm)?;
        return Ok(()); 
    }
    #[cfg(feature = "fatfs")]
    if parent.as_any().is::<crate::fs::fatfs::DirWrapper<'static, crate::dev::Disk>>() {
        return Ok(());
    }
    
    // 如果都不匹配，或文件系统不支持
    Err(axio::Error::from(AxError::Unsupported))
}

/// Creates a new symbolic link on the filesystem.
pub fn create_symlink(target: &str, link_path: &str) -> IoResult<()> {
    let (parent, link_name) = crate::root::lookup_parent(None, link_path)?;
    let any_node = parent.as_any();

    #[cfg(feature = "lwext4_rs")]
    if let Some(ext4_parent) = any_node.downcast_ref::<Ext4FileWrapper>() {
        // ext4 支持，调用底层函数
        return ext4_parent.0.lock().file_symlink(target, link_name)
            .map_err(|e| e.try_into().unwrap_or_default());
    }

    #[cfg(feature = "fatfs")]
    if any_node.is::<FatDirWrapper<'static, Disk>>() {
        // fatfs 不支持，直接返回一个 io::Error
        return Err(axio::Error::from(AxError::Unsupported));
    }

    Err(axio::Error::from(AxError::Unsupported))
}

/// Reads the value of a symbolic link.
pub fn read_link(path: &str) -> IoResult<String> {
    let node = crate::root::lookup(None, path)?;
    let any_node = node.as_any();

    #[cfg(feature = "lwext4_rs")]
    if let Some(ext4_node) = any_node.downcast_ref::<Ext4FileWrapper>() {
        let mut file = ext4_node.0.lock();
        // 确保是符号链接类型
        if file.get_type() != lwext4_rust::InodeTypes::EXT4_DE_SYMLINK {
            return Err(ax_err!(InvalidInput, "Not a symbolic link").into());
        }
        let mut buf = [0u8; 4096];
        let len = file.file_readlink(path, &mut buf)
            .map_err(|e| e.try_into().unwrap_or_default())?;
        return String::from_utf8(buf[..len].to_vec())
            .map_err(|_| ax_err!(InvalidData, "Invalid UTF8 in symlink").into());
    }
    
    #[cfg(feature = "fatfs")]
    if any_node.is::<FatDirWrapper<'static, Disk>>() || any_node.is::<FatFileWrapper<'static, Disk>>() {
        // fatfs 不支持，直接返回一个 io::Error
        return Err(axio::Error::from(AxError::Unsupported));
    }

    Err(axio::Error::from(AxError::Unsupported))
}

/// Changes the permissions of a file or directory.
pub fn set_permission(path: &str, perm: Permissions) -> IoResult<()> {
    let node = crate::root::lookup(None, path)?;
    let any_node = node.as_any();

    #[cfg(feature = "lwext4_rs")]
    if let Some(ext4_node) = any_node.downcast_ref::<Ext4FileWrapper>() {
        let mut file = ext4_node.0.lock();
        return file.file_chmod(path, perm.bits() as u32)
             .map_err(|e| e.try_into().unwrap_or_default());
    }

    #[cfg(feature = "fatfs")]
    if any_node.is::<FatFileWrapper<'static, Disk>>() || any_node.is::<FatDirWrapper<'static, Disk>>() {
        // 对于 fatfs，chmod 操作应该静默成功
        return Ok(());
    }

    Err(axio::Error::from(AxError::Unsupported))
}

/// Changes the owner and group of a file or directory.
pub fn set_owner(path: &str, uid: u32, gid: u32) -> IoResult<()> {
    let node = crate::root::lookup(None, path)?;
    let any_node = node.as_any();

    #[cfg(feature = "lwext4_rs")]
    if let Some(ext4_node) = any_node.downcast_ref::<Ext4FileWrapper>() {
        let mut file = ext4_node.0.lock();
        return file.file_chown(path, uid, gid)
            .map_err(|e| e.try_into().unwrap_or_default());
    }
    
    #[cfg(feature = "fatfs")]
    if any_node.is::<FatFileWrapper<'static, Disk>>() || any_node.is::<FatDirWrapper<'static, Disk>>() {
        // 对于 fatfs，chown 操作也应该静默成功
        return Ok(());
    }

    Err(axio::Error::from(AxError::Unsupported))
}