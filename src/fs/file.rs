// ====== ERROR ======

use std::{error, fmt, result};

#[derive(Debug)]
pub enum FdError {
}

impl error::Error for FdError {}

impl fmt::Display for FdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[FS] {:?}", &self)
    }
}

type Result<T> = result::Result<T, FdError>;

// ====== FD ======

use crate::logger;
use super::FsReq;
use super::metadata::Metadata;
use super::FdTable;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

/// File descriptor.
/// 
/// ## Methods
/// 
/// `read`: Read file content
/// 
/// `write`: Write content to file
pub struct Fd {
    inode: u32,
    meta: Metadata,
    tx: Sender<FsReq>,
    table: Arc<Mutex<FdTable>>,
}

impl Fd {
    /// Return a [u32] representing the virtual address of file inode.
    pub fn inode_addr(&self) -> u32 {
        self.inode
    }
    
    /// Return a [Metadata] wrapping the inode of the file.
    pub fn metadata(&mut self) -> &mut Metadata {
        &mut self.meta
    }

    /// Read file content. Return byte array [Vec]<[u8]> representing the content.
    pub fn read(&mut self) -> Result<Vec<u8>> {
        let (tx, rx) = mpsc::channel();
        if let Err(e) = self.tx.send(FsReq::ReadFile(tx, self.inode)) {
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

    /// Write content to file.
    /// 
    /// `data`: Content as byte array ([Vec]<[u8]>)
    pub fn write(&mut self, data: &Vec<u8>) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        if let Err(e) = self.tx.send(FsReq::WriteFile(tx, self.inode, data.clone())) {
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

impl Drop for Fd {
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

pub fn handle_open_file(tx: Sender<FsReq>, path: &str) -> Result<Fd> {
    todo!()
}

pub fn handle_create_file(tx: Sender<FsReq>, path: &str, uid: u8) -> Result<Fd> {
    todo!()
}

pub fn handle_remove_file(path: &str) -> Result<()> {
    todo!()
}

pub fn handle_read_file(inode: u32) -> Result<Vec<u8>> {
    todo!()
}

pub fn handle_write_file(inode: u32, data: &Vec<u8>) -> Result<()> {
    todo!()
}