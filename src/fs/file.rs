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

use super::FsReq;
use super::metadata::Metadata;
use std::sync::mpsc::{self, Sender, Receiver};

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
        todo!()
    }

    /// Write content to file.
    /// 
    /// `data`: Content as byte array ([Vec]<[u8]>)
    pub fn write(&mut self, data: &Vec<u8>) -> Result<()> {
        todo!()
    }
}