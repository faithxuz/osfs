// ====== ERROR =======

use std::{error, fmt};

#[derive(Debug)]
pub enum MetadataError {
    SystemTimeErr(std::time::SystemTimeError),
}

impl error::Error for MetadataError {}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetadataError: {:?}", self)
    }
}

impl From<std::time::SystemTimeError> for MetadataError {
    fn from(e: std::time::SystemTimeError) -> Self {
        Self::SystemTimeErr(e)
    }
}

type Result<T> = std::result::Result<T, MetadataError>;

// ====== METADATA ======

use crate::logger;
use super::inode;
use std::time;
use std::sync::{Arc, Mutex};

/// Permissons: read, write, execute
pub struct Rwx {
    read: bool,
    write: bool,
    execute: bool
}

impl Clone for Rwx {
    fn clone(&self) -> Self {
        Self { read: self.read, write: self.write, execute: self.execute }
    }
}

impl Copy for Rwx {}

pub struct Metadata {
    inode: Arc<Mutex<inode::Inode>>,
}

impl Metadata {
    pub fn is_dir(&self) -> bool {
        let lock = match self.inode.lock() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recovered from poisoned: {l:?}"));
                l
            }
        };
        lock.mode & inode::DIR_FLAG > 0
    }

    /// Return uid ([u8]) of file/directory owner.
    pub fn owner(&self) -> u8 {
        let lock = match self.inode.lock() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recovered from poisoned: {l:?}"));
                l
            }
        };
        lock.uid
    }

    /// Return [Rwx] unit: (owner_permission, others_permission)
    pub fn permission(&self) -> (Rwx, Rwx) {
        let lock = match self.inode.lock() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recovered from poisoned: {l:?}"));
                l
            }
        };
        (
            Rwx {
                read: lock.mode & inode::OWNER_RWX_FLAG.0 > 0,
                write: lock.mode & inode::OWNER_RWX_FLAG.1 > 0,
                execute: lock.mode & inode::OWNER_RWX_FLAG.2 > 0
            },
            Rwx {
                read: lock.mode & inode::OTHER_RWX_FLAG.0 > 0,
                write: lock.mode & inode::OTHER_RWX_FLAG.1 > 0,
                execute: lock.mode & inode::OTHER_RWX_FLAG.2 > 0
            }
        )
    }

    /// `permission`: [Rwx] unit: (owner_permission, others_permission)
    pub fn set_permission(&mut self, permission: (Rwx, Rwx)) {
        let mut lock = match self.inode.lock() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recovered from poisoned: {l:?}"));
                l
            }
        };
        match permission.0.read {
            true => lock.mode |= inode::OWNER_RWX_FLAG.0,
            false => lock.mode |= !inode::OWNER_RWX_FLAG.0
        }
        match permission.0.write {
            true => lock.mode |= inode::OWNER_RWX_FLAG.1,
            false => lock.mode |= !inode::OWNER_RWX_FLAG.1
        }
        match permission.0.execute {
            true => lock.mode |= inode::OWNER_RWX_FLAG.2,
            false => lock.mode |= !inode::OWNER_RWX_FLAG.2
        }

        match permission.1.read {
            true => lock.mode |= inode::OTHER_RWX_FLAG.0,
            false => lock.mode |= !inode::OTHER_RWX_FLAG.0
        }
        match permission.1.write {
            true => lock.mode |= inode::OTHER_RWX_FLAG.1,
            false => lock.mode |= !inode::OTHER_RWX_FLAG.1
        }
        match permission.1.execute {
            true => lock.mode |= inode::OTHER_RWX_FLAG.2,
            false => lock.mode |= !inode::OTHER_RWX_FLAG.2
        }
    }

    /// Update to now
    pub fn update_timestamp(&mut self) -> Result<()> {
        let mut lock = match self.inode.lock() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recovered from poisoned: {l:?}"));
                l
            }
        };
        lock.timestamp = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)?
            .as_secs() as u32;
        Ok(())
    }
}