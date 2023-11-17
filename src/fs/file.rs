// ====== ERROR ======

use std::{error, fmt, result, io, sync::mpsc};
use super::error::*;

#[derive(Debug)]
pub enum FdError {
    SendErr(String),
    RecvErr(String),
    InvalidPath,
    NotFound,
    NotFile,
    ParentNotFound,
    ParentNotDir,
    FileExists,
    FileOccupied,
    FileIncorrupted,
    NoEnoughSpace,
    IoErr(io::Error),
}

impl error::Error for FdError {}

impl fmt::Display for FdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[FS] {:?}", &self)
    }
}

impl From<mpsc::SendError<FsReq>> for FdError {
    fn from(e: mpsc::SendError<FsReq>) -> Self { Self::SendErr(format!("{e:?}")) }
}

impl From<mpsc::RecvError> for FdError {
    fn from(e: mpsc::RecvError) -> Self { Self::RecvErr(format!("{e:?}")) }
}

impl From<io::Error> for FdError {
    fn from(e: io::Error) -> Self { Self::IoErr(e) }
}

impl From<DiskError> for FdError {
    fn from(e: DiskError) -> Self {
        match e {
            DiskError::InvalidAddr => return Self::FileIncorrupted,
            DiskError::IoErr(e) => return Self::IoErr(e)
        }
    }
}

impl From<InodeError> for FdError {
    fn from(e: InodeError) -> Self {
        match e {
            InodeError::InvalidAddr => return Self::NotFound,
            InodeError::NoUsableBlock => return Self::NoEnoughSpace,
            InodeError::DataTooBig => return Self::NoEnoughSpace,
            InodeError::DiskErr(e) => return Self::DiskErr(e),
        }
    }
}

impl From<DataError> for FdError {
    fn from(e: DataError) -> Self {
        match e {
            DataError::InvalidAddr => return Self::FileIncorrupted,
            DataError::InsufficientUsableBlocks => return Self::NoEnoughSpace,
            DataError::DiskErr(e) => return Self::DiskErr(e),
        }
    }
}

impl FdError {
    fn DiskErr(e: DiskError) -> Self {
        match e {
            DiskError::InvalidAddr => return Self::FileIncorrupted,
            DiskError::IoErr(e) => return Self::IoErr(e)
        }
    }
}

type Result<T> = result::Result<T, FdError>;

// ====== FD ======

use crate::logger;
use super::{FsReq, FdTable, metadata::Metadata};
use super::utils;
use std::sync::mpsc::Sender;
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
        self.tx.send(FsReq::ReadFile(tx, self.inode))?;
        match rx.recv()? {
            Ok(data) => Ok(data),
            Err(e) => return Err(FdError::NotFound)
        }
    }

    /// Write content to file.
    /// 
    /// `data`: Content as byte array ([Vec]<[u8]>)
    pub fn write(&mut self, data: &Vec<u8>) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(FsReq::WriteFile(tx, self.inode, data.clone()))?;
        match rx.recv()? {
            Ok(_) => Ok(()),
            Err(e) => return Err(FdError::NotFound)
        }
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        let mut lock = utils::mutex_lock(self.table.lock());
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
            FsError::InvalidPath => return Err(FdError::InvalidPath),
            FsError::NotFound => return Err(FdError::NotFound),
            _ => panic!("{e:?}")
        }
    };
    let inode = inode::load_inode(inode_addr)?;
    let metadata = Metadata::new(inode_addr, inode, tx.clone());
    if metadata.is_dir() {
        return Err(FdError::NotFile);
    }

    // add into fd table
    let mut lock = utils::mutex_lock(fd_table.lock());
    let fd = match lock.get_file(tx.clone(), inode_addr, fd_table.clone()) {
        Ok(f) => f,
        Err(e) => match e {
            FsError::NotDirButFile => return Err(FdError::NotFile),
            _ => panic!("{e:?}")
        }
    };

    logger::log(&format!("[FS] Open file: {path}"));
    Ok(fd)
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
            DdError::NotFound => return Err(FdError::ParentNotFound),
            DdError::NotDir => return Err(FdError::ParentNotDir),
            _ => /**/panic!("{e:?}")
        }
    };
    match metadata(tx.clone(), path) {
        Ok(_) => return Err(FdError::FileExists),
        Err(e) => match e {
            MetadataError::InvalidPath => return Err(FdError::InvalidPath),
            MetadataError::NotFound => (),
            MetadataError::DiskErr(e) => return Err(FdError::DiskErr(e)),
            _ => panic!("{e:?}")
        }
    }

    let mut inode = inode::alloc_inode(uid, false)?;

    // add parent/new
    if let Err(e) = dir::dir_add_entry(parent_dd.inode_addr(), inode.0, &dir_name) {
        match e {
            DdError::DirIncorrupted => return Err(FdError::FileIncorrupted),
            DdError::NoEnoughSpace => return Err(FdError::NoEnoughSpace),
            DdError::EntryExists => return Err(FdError::FileExists),
            DdError::IoErr(e) => return Err(FdError::IoErr(e)),
            _ => panic!("{e:?}")
        }
    }

    // alloc data block
    let blocks = data::alloc_blocks(1)?;

    // write a EOF
    let data = [(
        *blocks.get(0).unwrap(),
        [0u8].to_vec()
    )].to_vec();
    disk::write_blocks(&data)?;

    // update and save inode
    inode::update_blocks(&mut inode.1, &blocks)?;
    inode::save_inode(inode.0, &inode.1)?;

    // add into fd table
    let mut lock = utils::mutex_lock(fd_table.lock());
    let fd = match lock.get_file(tx.clone(), inode.0, fd_table.clone()) {
        Ok(f) => f,
        Err(e) => match e {
            FsError::NotDirButFile => return Err(FdError::NotFile),
            _ => panic!("{e:?}")
        }
    };

    logger::log(&format!("[FS] Create file by user{uid}: {path}"));
    Ok(fd)
}

/// ## Error
/// 
/// - InvalidPath
/// - NotFound
/// - NotFile
/// - FileOccupied
/// - FileIncorrupted
/// - ParentNotFound
/// - ParentNotDir
/// - IoErr
pub fn remove_file(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str) -> Result<()> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::InvalidPath => return Err(FdError::InvalidPath),
            FsError::NotFound => return Err(FdError::NotFound),
            _ => panic!("{e:?}")
        }
    };
    let inode = inode::load_inode(inode_addr)?;
    let metadata = Metadata::new(inode_addr, inode, tx.clone());
    if metadata.is_dir() {
        return Err(FdError::NotFile);
    }

    {
        let lock = utils::mutex_lock(fd_table.lock());
        if let Some(_) = lock.check(inode_addr) {
            return Err(FdError::FileOccupied);
        }
    }

    let mut path_vec: Vec<&str> = path.split('/').collect();
    path_vec.drain(0..1);
    path_vec.pop();
    let parent_path = String::from("/") + &path_vec.join("/");
    let parent_dd = match dir::open_dir(tx.clone(), fd_table.clone(), &parent_path) {
        Ok(d) => d,
        Err(e) => match e {
            DdError::NotFound => return Err(FdError::ParentNotFound),
            DdError::NotDir => return Err(FdError::ParentNotDir),
            _ => panic!("{e:?}")
        }
    };

    // remove entry from parent directory
    if let Err(e) = dir::dir_remove_entry(parent_dd.inode_addr(), inode_addr) {
        match e {
            DdError::DirIncorrupted => return Err(FdError::FileIncorrupted),
            DdError::IoErr(e) => return Err(FdError::IoErr(e)),
            _ => panic!("{e:?}")
        }
    }

    // remove file data
    let inode = inode::load_inode(inode_addr)?;
    let blocks = inode::get_blocks(&inode)?;
    data::free_blocks(&blocks)?;

    // free inode
    inode::free_inode(inode_addr)?;

    logger::log(&format!("[FS] Remove file: {path}"));
    Ok(())
}

/// ## Error
/// 
/// - NotFound
/// - FileIncorrupted
/// - IoErr
pub fn read_file(inode: u32) -> Result<Vec<u8>> {
    let inode = inode::load_inode(inode)?;
    let blocks = inode::get_blocks(&inode)?;
    let mut buf = disk::read_blocks(&blocks)?;

    // trim end
    let end_at = 0;
    for b in &buf {
        if *b == 0 {
            break
        }
    }
    buf.drain(end_at..);
    
    Ok(buf)
}

/// ## Error
/// 
/// - NotFound
/// - NoEnoughSpace
/// - FileIncorrupted
/// - IoErr
pub fn write_file(inode_addr: u32, buf: &mut Vec<u8>) -> Result<()> {
    buf.push(0);
    let blocks_len = buf.len().div_ceil(disk::BLOCK_SIZE as usize);
    let mut inode = inode::load_inode(inode_addr)?;
    let mut blocks = inode::get_blocks(&inode)?;

    if blocks.len() > blocks_len {
        // free
        let to_free_count = blocks.len() - blocks_len;
        let mut to_free = Vec::<u32>::with_capacity(to_free_count);
        for _ in 0..to_free_count {
            to_free.push(blocks.pop().unwrap());
        }
        data::free_blocks(&to_free)?;
        inode::update_blocks(&mut inode, &blocks)?;
    } else if blocks.len() < blocks_len {
        // alloc
        let mut to_append = data::alloc_blocks((blocks_len - blocks.len()) as u32)?;
        blocks.append(&mut to_append);
        inode::update_blocks(&mut inode, &blocks)?;
    }
    inode::save_inode(inode_addr, &inode)?;

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
    disk::write_blocks(&data)?;

    Ok(())
}