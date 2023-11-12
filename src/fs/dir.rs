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

use crate::logger;
use super::FsReq;
use super::metadata::Metadata;
use super::FdTable;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

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
    table: Arc<Mutex<FdTable>>,
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
        let (tx, rx) = mpsc::channel();
        if let Err(e) = self.tx.send(FsReq::ReadDir(tx, self.inode)) {
            todo!()
        }
        match rx.recv() {
            Ok(res) => match res {
                Ok(data) => Ok(data),
                Err(e) => { todo!() }
            },
            Err(e) => { todo!() }
        }
    }

    /// Add an entry to directory.
    /// 
    /// `inode`: virtual address of the inode that entry represents.
    /// 
    /// `name`: file name.
    pub fn add_entry(&mut self, inode: u32, name: &str) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        if let Err(e) = self.tx.send(FsReq::DirAddEntry(tx, self.inode, inode, name.to_string())) {
            todo!()
        }
        match rx.recv() {
            Ok(res) => match res {
                Ok(_) => Ok(()),
                Err(e) => { todo!() }
            },
            Err(e) => { todo!() }
        }
    }

    /// Remove an entry from directory.
    /// 
    /// `inode`: virtual address of the inode that entry represents.
    pub fn remove_entry(&mut self, inode: u32) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        if let Err(e) = self.tx.send(FsReq::DirRemoveEntry(tx, self.inode, inode)) {
            todo!()
        }
        match rx.recv() {
            Ok(res) => match res {
                Ok(_) => Ok(()),
                Err(e) => { todo!() }
            },
            Err(e) => { todo!() }
        }
    }
}

impl Drop for Dd {
    fn drop(&mut self) {
        let mut lock = match self.table.lock() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recovered from poisoned: {l:?}"));
                l
            }
        };
        lock.try_drop(self.inode);
    }
}

// ====== FN ======

pub fn handle_open_dir(tx: Sender<FsReq>, path: &str) -> Result<Dd> {
    todo!()
}

pub fn handle_create_dir(tx: Sender<FsReq>, path: &str, uid: u8) -> Result<Dd> {
    todo!()
}

pub fn handle_remove_dir(path: &str) -> Result<()> {
    todo!()
}

pub fn handle_read_dir(dir_inode: u32) -> Result<Vec<Entry>> {
    todo!()
}

pub fn handle_dir_add_entry(dir_inode: u32, entry_inode: u32, name: &str) -> Result<()> {
    todo!()
}

pub fn handle_dir_remove_entry(dir_inode: u32, entry_inode: u32) -> Result<()> {
    todo!()
}