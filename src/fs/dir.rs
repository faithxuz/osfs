// ====== ERROR ======

use std::{error, fmt, result};

#[derive(Debug)]
pub enum DdError {
}

impl error::Error for DdError {}

impl fmt::Display for DdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[FS] {:?}", &self)
    }
}

type Result<T> = result::Result<T, DdError>;

// ====== DD ======

use super::FsReq;
use super::metadata::Metadata;
use std::sync::mpsc::{self, Sender, Receiver};

/// Entry in directory. Each has:
/// 
/// `inode`: virtual address of inode
/// 
/// `name`: file name
pub struct Entry {
    inode: u32,
    name: String,
}

/// Directory descriptor.
/// 
/// ## Methods
/// 
/// `read`: Read entries in directory
/// 
/// `add_entry`: Add an entry to directory
/// 
/// `remove_entry`: Remove an entry from directory
pub struct Dd {
    inode: u32,
    meta: Metadata,
    tx: Sender<FsReq>,
}

impl Dd {
    /// Return a [u32] representing the virtual address of directory inode.
    pub fn inode_addr(&self) -> u32 {
        crate::logger::log("a");
        self.inode
    }

    /// Return a [Metadata] wrapping the inode of the directory.
    pub fn metadata(&mut self) -> &mut Metadata {
        &mut self.meta
    }

    /// Read entries in directory. Return [Vec]<[Entry]>.
    pub fn read(&mut self) -> Result<Vec<Entry>> {
        todo!()
    }

    /// Add an entry to directory.
    /// 
    /// `inode`: virtual address of the inode that entry represents.
    /// 
    /// `name`: file name.
    pub fn add_entry(&mut self, inode: u32, name: &str) -> Result<()> {
        todo!()
    }

    /// Remove an entry from directory.
    /// 
    /// `inode`: virtual address of the inode that entry represents.
    pub fn remove_entry(&mut self, inode: u32) -> Result<()> {
        todo!()
    }
}