// ====== ERROR =======

use std::{error, fmt, sync::mpsc};
use super::error::*;
use super::disk;

#[derive(Debug)]
pub enum MetadataError {
    RecvErr(String),
    InvalidPath,
    NotFound,
    DiskErr(disk::DiskError),
}

impl error::Error for MetadataError {}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetadataError: {:?}", self)
    }
}

impl From<disk::DiskError> for MetadataError {
    fn from(e: disk::DiskError) -> Self { Self::DiskErr(e) }
}

impl From<mpsc::RecvError> for MetadataError {
    fn from(e: mpsc::RecvError) -> Self { Self::RecvErr(format!("{e:?}")) }
}

type Result<T> = std::result::Result<T, MetadataError>;

// ====== METADATA ======

use crate::logger;
use super::FsReq;
use super::inode;
use std::sync::mpsc::Sender;
use chrono::prelude::*;

/// Permissons: read, write, execute
#[derive(Debug)]
pub struct Rwx {
    pub read: bool,
    pub write: bool,
    pub execute: bool
}

impl Clone for Rwx {
    fn clone(&self) -> Self {
        Self { read: self.read, write: self.write, execute: self.execute }
    }
}

impl Copy for Rwx {}

pub struct Metadata {
    addr: u32,
    inode: inode::Inode,
    tx: Sender<FsReq>,
}

impl Metadata {
    pub fn new(addr: u32, inode: inode::Inode, tx: Sender<FsReq>) -> Self {
        Self { addr, inode, tx }
    }

    /// Return `ture` if being a directory; `false` for being file.
    pub fn is_dir(&self) -> bool {
        self.inode.mode & inode::DIR_FLAG > 0
    }

    /// Return uid ([u8]) of file/directory owner.
    pub fn owner(&self) -> u8 {
        self.inode.uid
    }

    // Return file/directory size in bytes.
    pub fn size(&self) -> u32 {
        self.inode.size
    }

    /// Return [Rwx] unit: (owner_permission, others_permission)
    pub fn permission(&self) -> (Rwx, Rwx) {
        (
            Rwx {
                read: self.inode.mode & inode::OWNER_RWX_FLAG.0 > 0,
                write: self.inode.mode & inode::OWNER_RWX_FLAG.1 > 0,
                execute: self.inode.mode & inode::OWNER_RWX_FLAG.2 > 0
            },
            Rwx {
                read: self.inode.mode & inode::OTHER_RWX_FLAG.0 > 0,
                write: self.inode.mode & inode::OTHER_RWX_FLAG.1 > 0,
                execute: self.inode.mode & inode::OTHER_RWX_FLAG.2 > 0
            }
        )
    }

    /// `permission`: [Rwx] unit: (owner_permission, others_permission)
    pub fn set_permission(&mut self, permission: (Rwx, Rwx)) -> Result<()> {
        match permission.0.read {
            true => self.inode.mode |= inode::OWNER_RWX_FLAG.0,
            false => self.inode.mode |= !inode::OWNER_RWX_FLAG.0
        }
        match permission.0.write {
            true => self.inode.mode |= inode::OWNER_RWX_FLAG.1,
            false => self.inode.mode |= !inode::OWNER_RWX_FLAG.1
        }
        match permission.0.execute {
            true => self.inode.mode |= inode::OWNER_RWX_FLAG.2,
            false => self.inode.mode |= !inode::OWNER_RWX_FLAG.2
        }

        match permission.1.read {
            true => self.inode.mode |= inode::OTHER_RWX_FLAG.0,
            false => self.inode.mode |= !inode::OTHER_RWX_FLAG.0
        }
        match permission.1.write {
            true => self.inode.mode |= inode::OTHER_RWX_FLAG.1,
            false => self.inode.mode |= !inode::OTHER_RWX_FLAG.1
        }
        match permission.1.execute {
            true => self.inode.mode |= inode::OTHER_RWX_FLAG.2,
            false => self.inode.mode |= !inode::OTHER_RWX_FLAG.2
        }

        let (tx, rx) = mpsc::channel();
        self.tx.send(FsReq::UpdateInode(tx, self.addr, self.inode)).unwrap();
        match rx.recv()? {
            Ok(_) => {
                logger::log(&format!("Update permission for inode {}.", self.addr));
                Ok(())
            },
            Err(e) => todo!()
        }
    }

    /// Return unit (month, date, hour, minute).
    /// 
    /// Note: month starts from 0
    pub fn timestamp(&self) -> (u32, u32, u32, u32) {
        let dt = DateTime::from_timestamp(self.inode.timestamp as i64, 0).unwrap();
        (dt.month0(), dt.day(), dt.hour(), dt.minute())
    }

    /// Update to now
    pub fn update_timestamp(&mut self) -> Result<()> {
        self.inode.update_timestamp();

        let (tx, rx) = mpsc::channel();
        self.tx.send(FsReq::UpdateInode(tx, self.addr, self.inode)).unwrap();
        match rx.recv()? {
            Ok(_) => {
                logger::log(&format!("Update timestamp for inode {}.", self.addr));
                Ok(())
            },
            Err(e) => todo!()
        }
    }
}

// ====== FN =======

use super::FsError;
use super::path_to_inode;

/// ## Error
/// 
/// - InvalidAddr
/// - NotFound
/// - DiskErr
pub fn metadata(tx: Sender<FsReq>, path: &str) -> Result<Metadata> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::NotFound => return Err(MetadataError::NotFound),
            FsError::InvalidPath => return Err(MetadataError::InvalidPath),
            _ => panic!("{e:?}")
        }
    };
    let inode = match inode::load_inode(inode_addr) {
        Ok(i) => i,
        Err(e) => match e {
            InodeError::InvalidAddr => return Err(MetadataError::NotFound),
            InodeError::DiskErr(e) => return Err(MetadataError::DiskErr(e)),
            _ => panic!("{e:?}")
        }
    };

    logger::log(&format!("Get metadata of \"{path}\""));
    Ok(Metadata::new(inode_addr, inode, tx))
}