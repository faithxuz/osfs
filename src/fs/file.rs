// ====== ERROR ======

use std::{error, fmt, result};

#[derive(Debug)]
pub enum FdError {
    InvalidPath,
    NotFound,
    NotFile,
    ParentNotFound,
    ParentNotDir,
    FileExists,
    FileOccupied,
    NoEnoughSpace,
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
    pub fn new(inode: u32, meta: Metadata, tx: Sender<FsReq>, table: Arc<Mutex<FdTable>>) -> Self {
        Self { inode, meta, tx, table }
    }

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
                Err(e) => todo!()
            },
            Err(e) => todo!()
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
                Err(e) => todo!()
            },
            Err(e) => todo!()
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

use super::FsError;
use super::{disk, inode, data, dir};
use super::{path_to_inode, metadata::metadata};

pub fn open_file(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str) -> Result<Fd> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::NotFound => return Err(FdError::NotFound),
            FsError::NotFileButDir => return Err(FdError::NotFile),
            _ => todo!()
        }
    };
    let inode = match inode::load_inode(inode_addr) {
        Ok(i) => i,
        Err(e) => match e {
            inode::InodeError::InvalidAddr => return Err(FdError::NotFound),
            _ => todo!()
        }
    };
    let metadata = Metadata::new(inode_addr, inode, tx.clone());

    // add into fd table
    let mut lock = match fd_table.lock() {
        Ok(l) => l,
        Err(poisoned) => {
            let l = poisoned.into_inner();
            logger::log(&format!("Recovered from poisoned: {l:?}"));
            l
        }
    };
    match lock.get_file(inode_addr) {
        Ok(opt) => if let None = opt {
            if let Err(e) = lock.add_file(inode_addr, &inode) {
                todo!()
            }
        },
        Err(e) => todo!()
    }

    logger::log(&format!("Open file: {path}"));
    Ok(Fd::new(inode_addr, metadata, tx, fd_table.clone()))
}

pub fn create_file(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str, uid: u8) -> Result<Fd> {
    let mut path_vec: Vec<&str> = path.split('/').collect();
    let dir_name = String::from(match path_vec.pop() {
        Some(n) => n,
        None => return Err(FdError::InvalidPath)
    });
    let parent_path = path_vec.join("/");
    let parent_dd = match dir::open_dir(tx.clone(), fd_table.clone(), &parent_path) {
        Ok(d) => d,
        Err(e) => match e {
            dir::DdError::NotFound => return Err(FdError::ParentNotFound),
            dir::DdError::NotDir => return Err(FdError::ParentNotDir),
            _ => todo!()
        }
    };
    if let Ok(_) = metadata(tx.clone(), path) {
        return Err(FdError::FileExists);
    }

    let mut inode = match inode::alloc_inode(uid, false) {
        Ok(i) => i,
        Err(e) => todo!()
    };
    let metadata = Metadata::new(inode.0, inode.1, tx.clone());

    // add parent/new
    if let Err(e) = dir::dir_add_entry(parent_dd.inode_addr(), inode.0, &dir_name) {
        todo!()
    }

    // alloc data block
    let blocks = match data::alloc_blocks(1) {
        Ok(v) => v,
        Err(e) => todo!()
    };

    // write a EOF
    let data = [(match blocks.get(0) {
        Some(a) => *a,
        None => todo!()
    }, [0u8].to_vec())].to_vec();
    if let Err(e) = disk::write_blocks(&data) {
        todo!()
    }

    // update and save inode
    if let Err(e) = inode::update_blocks(&mut inode.1, &blocks) {
        todo!()
    }
    if let Err(e) = inode::save_inode(inode.0, &inode.1) {
        todo!()
    }

    // add into fd table
    let mut lock = match fd_table.lock() {
        Ok(l) => l,
        Err(poisoned) => {
            let l = poisoned.into_inner();
            logger::log(&format!("Recovered from poisoned: {l:?}"));
            l
        }
    };
    if let Err(e) = lock.add_file(inode.0, &inode.1) {
        todo!()
    }

    logger::log(&format!("Create file by user{uid}: {path}"));
    Ok(Fd::new(inode.0, metadata, tx.clone(), fd_table.clone()))
}

pub fn remove_file(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str) -> Result<()> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::NotFound => return Err(FdError::NotFound),
            FsError::NotFileButDir => return Err(FdError::NotFile),
            _ => todo!()
        }
    };

    let mut lock = match fd_table.lock() {
        Ok(l) => l,
        Err(poisoned) => {
            let l = poisoned.into_inner();
            logger::log(&format!("Recovered from poisoned: {l:?}"));
            l
        }
    };
    if let Ok(_) = lock.get_dir(inode_addr) {
        return Err(FdError::FileOccupied);
    }

    let path_vec: Vec<&str> = path.split('/').collect();
    let parent_path = path_vec[..path_vec.len()-1].join("/");
    let mut parent_dd = match dir::open_dir(tx.clone(), fd_table.clone(), &parent_path) {
        Ok(d) => d,
        Err(e) => match e {
            dir::DdError::NotFound => return Err(FdError::ParentNotFound),
            dir::DdError::NotDir => return Err(FdError::ParentNotDir),
            _ => todo!()
        }
    };
    if let Err(e) = metadata(tx.clone(), path) {
        return Err(FdError::NotFound);
    }

    // remove entry from parent directory
    if let Err(e) = parent_dd.remove_entry(inode_addr) {
        todo!()
    };

    // remove file data
    let inode = match inode::load_inode(inode_addr) {
        Ok(i) => i,
        Err(e) => todo!()
    };
    let blocks = match inode::get_blocks(&inode) {
        Ok(v) => v,
        Err(e) => todo!()
    };
    if let Err(e) = data::free_blocks(&blocks) {
        todo!()
    }

    // free inode
    if let Err(e) = inode::free_inode(inode_addr) {
        todo!()
    };

    logger::log(&format!("Remove file: {path}"));
    Ok(())
}

pub fn read_file(inode: u32) -> Result<Vec<u8>> {
    let inode = match inode::load_inode(inode) {
        Ok(i) => i,
        Err(e) => match e {
            inode::InodeError::InvalidAddr => return Err(FdError::NotFound),
            _ => todo!()
        }
    };
    let blocks = match inode::get_blocks(&inode) {
        Ok(b) => b,
        Err(e) => todo!()
    };
    let mut buf = match disk::read_blocks(&blocks) {
        Ok(b) => b,
        Err(e) => todo!()
    };

    // trim end
    let end_at = 0;
    for b in &buf {
        if *b == 0 {
            break
        }
    }
    buf = buf.drain(end_at..).collect();
    
    Ok(buf)
}

pub fn write_file(inode_addr: u32, buf: &Vec<u8>) -> Result<()> {
    let blocks_len = buf.len().div_ceil(disk::BLOCK_SIZE as usize);
    let mut inode = match inode::load_inode(inode_addr) {
        Ok(i) => i,
        Err(e) => todo!()
    };
    let mut blocks = match inode::get_blocks(&inode) {
        Ok(v) => v,
        Err(e) => todo!()
    };

    if blocks.len() > blocks_len {
        // free
        let to_free_count = blocks.len() - blocks_len;
        let mut to_free = Vec::<u32>::with_capacity(to_free_count);
        for _ in 0..to_free_count {
            to_free.push(blocks.pop().unwrap());
        }
        if let Err(e) = data::free_blocks(&to_free) {
            todo!()
        }
        if let Err(e) = inode::update_blocks(&mut inode, &blocks) {
            todo!()
        }
    } else if blocks.len() < blocks_len {
        // alloc
        let mut to_append = match data::alloc_blocks((blocks_len - blocks.len()) as u32) {
            Ok(v) => v,
            Err(e) => todo!()
        };
        blocks.append(&mut to_append);
        if let Err(e) = inode::update_blocks(&mut inode, &blocks) {
            todo!()
        }
    }
    if let Err(e) = inode::save_inode(inode_addr, &inode) {
        todo!()
    }

    if blocks_len == 0 {
        return Ok(());
    }

    let buf = &buf[..];
    let mut data = Vec::<(u32, Vec<u8>)>::with_capacity(blocks_len);
    let last_block = blocks.pop().unwrap();
    for (i, addr) in blocks.iter().enumerate() {
        data.push((*addr, buf[i*disk::BLOCK_SIZE as usize..(i+1)*disk::BLOCK_SIZE as usize].to_vec()));
    }
    data.push((last_block, buf[(blocks_len-1)*disk::BLOCK_SIZE as usize..].to_vec()));
    if let Err(e) = disk::write_blocks(&data) {
        todo!()
    }

    Ok(())
}