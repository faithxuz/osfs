// ====== ERROR ======

use std::{error, fmt, result, io};
use super::error::*;

#[derive(Debug)]
pub enum DdError {
    SendErr(String),
    RecvErr(String),
    InvalidPath,
    NotFound,
    NotDir,
    ParentNotFound,
    ParentNotDir,
    DirExists,
    DirOccupied,
    DirIncorrupted,
    NoEnoughSpace,
    EntryExists,
    IoErr(io::Error),
}

impl error::Error for DdError {}

impl fmt::Display for DdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[FS] {:?}", &self)
    }
}

impl From<mpsc::SendError<FsReq>> for DdError {
    fn from(e: mpsc::SendError<FsReq>) -> Self { Self::SendErr(format!("{e:?}")) }
}

impl From<mpsc::RecvError> for DdError {
    fn from(e: mpsc::RecvError) -> Self { Self::RecvErr(format!("{e:?}")) }
}

impl From<io::Error> for DdError {
    fn from(e: io::Error) -> Self { Self::IoErr(e) }
}

impl From<DiskError> for DdError {
    fn from(e: DiskError) -> Self {
        match e {
            DiskError::InvalidAddr => return Self::DirIncorrupted,
            DiskError::IoErr(e) => return Self::IoErr(e)
        }
    }
}

impl From<InodeError> for DdError {
    fn from(e: InodeError) -> Self {
        match e {
            InodeError::InvalidAddr => return Self::NotFound,
            InodeError::NoUsableBlock => return Self::NoEnoughSpace,
            InodeError::DataTooBig => return Self::NoEnoughSpace,
            InodeError::DiskErr(e) => return Self::DiskErr(e),
        }
    }
}

impl From<DataError> for DdError {
    fn from(e: DataError) -> Self {
        match e {
            DataError::InvalidAddr => return Self::DirIncorrupted,
            DataError::InsufficientUsableBlocks => return Self::NoEnoughSpace,
            DataError::DiskErr(e) => return Self::DiskErr(e),
        }
    }
}

impl From<FdError> for DdError {
    fn from(e: FdError) -> Self {
        match e {
            FdError::NotFound => return DdError::NotFound,
            FdError::FileIncorrupted => return DdError::DirIncorrupted,
            FdError::NoEnoughSpace => return DdError::NoEnoughSpace,
            FdError::IoErr(e) => return DdError::IoErr(e),
            _ => panic!("{e:?}")
        }
    }
}

impl DdError {
    fn DiskErr(e: DiskError) -> Self {
        match e {
            DiskError::InvalidAddr => return Self::DirIncorrupted,
            DiskError::IoErr(e) => return Self::IoErr(e)
        }
    }
}

type Result<T> = result::Result<T, DdError>;

// ====== DD ======

use crate::logger;
use crate::sedes::{Serialize, Deserialize};
use super::{utils, metadata::Metadata};
use super::{FdTable, FsReq, FsError};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

pub const ENTRY_SIZE: usize = 64;
pub const NAME_LEN: usize = ENTRY_SIZE - 4;

/// Entry in directory. Each has:
/// 
/// `inode`: virtual address of inode
/// 
/// `name`: file name
#[derive(Debug, Default)]
pub struct Entry {
    pub inode: u32,     // 4
    pub name: String,   // 60
}

impl Serialize for Entry {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::<u8>::with_capacity(ENTRY_SIZE);
        v.append(&mut utils::u32_to_u8arr(self.inode).to_vec());
        let mut name: Vec<u8>;
        if self.name.len() > NAME_LEN {
            name = (*&self.name[0..NAME_LEN].to_string()).clone().into_bytes();
        } else {
            name = self.name.as_bytes().to_vec();
        }
        v.append(&mut name);
        v
    }
}

impl Deserialize for Entry {
    fn deserialize(buf: &mut Vec<u8>) -> result::Result<Self, SedesError> where Self: Sized {
        if buf.len() < ENTRY_SIZE {
            return Err(SedesError::DeserialBufferTooSmall);
        }
        let mut me = Self::default();
        me.inode = utils::u8arr_to_u32(&buf[0..4]);
        me.name = String::from_utf8(buf[4..].to_vec()).unwrap();
        Ok(me)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.inode == other.inode && self.name == other.name
    }
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
    pub fn new(inode: u32, meta: Metadata, tx: Sender<FsReq>, table: Arc<Mutex<FdTable>>) -> Self {
        Self { inode, meta, tx, table }
    }

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
        self.tx.send(FsReq::ReadDir(tx, self.inode))?;
        match rx.recv()? {
            Ok(data) => Ok(data),
            Err(e) => todo!()
        }
    }

    /// Add an entry to directory.
    /// 
    /// `inode`: virtual address of the inode that entry represents.
    /// 
    /// `name`: file name.
    pub fn add_entry(&mut self, inode: u32, name: &str) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(FsReq::DirAddEntry(tx, self.inode, inode, name.to_string()))?;
        match rx.recv()? {
            Ok(_) => Ok(()),
            Err(e) => todo!()
        }
    }

    /// Remove an entry from directory.
    /// 
    /// `inode`: virtual address of the inode that entry represents.
    pub fn remove_entry(&mut self, inode: u32) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(FsReq::DirRemoveEntry(tx, self.inode, inode))?;
        match rx.recv()? {
            Ok(_) => Ok(()),
            Err(e) => todo!()
        }
    }
}

impl Drop for Dd {
    fn drop(&mut self) {
        let mut lock = utils::mutex_lock(self.table.lock());
        lock.try_drop(self.inode);
    }
}

// ====== FN ======

use super::{inode, data, file, metadata::metadata};
use super::FdError;
use super::path_to_inode;

/// ## Error
/// 
/// - InvalidPath
/// - NotFound
/// - NotDir
/// - DirIncorrupted
pub fn open_dir(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str) -> Result<Dd> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::InvalidPath => return Err(DdError::InvalidPath),
            FsError::NotFound => return Err(DdError::NotFound),
            _ => panic!("{e:?}")
        }
    };
    let inode = inode::load_inode(inode_addr)?;
    let metadata = Metadata::new(inode_addr, inode, tx.clone());
    if !metadata.is_dir() {
        return Err(DdError::NotDir);
    }

    // add into fd table
    let mut lock = utils::mutex_lock(fd_table.lock());
    match lock.get_dir(inode_addr) {
        Ok(opt) => if let None = opt {
            lock.add_dir(inode_addr, &inode).unwrap();
        },
        Err(e) => match e {
            FsError::NotDirButFile => return Err(DdError::DirIncorrupted),
            _ => panic!("{e:?}")
        }
    }

    logger::log(&format!("Open directory: {path}"));
    Ok(Dd::new(inode_addr, metadata, tx, fd_table.clone()))
}

/// ## Error
/// 
/// - InvalidPath
/// - ParentNotFound
/// - ParentNotDir
/// - DirExists
pub fn create_dir(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str, uid: u8) -> Result<Dd> {
    let mut path_vec: Vec<&str> = path.split('/').collect();
    let dir_name = String::from(match path_vec.pop() {
        Some(n) => n,
        None => return Err(DdError::InvalidPath)
    });
    let parent_path = path_vec.join("/");
    let parent_dd = match open_dir(tx.clone(), fd_table.clone(), &parent_path) {
        Ok(d) => d,
        Err(e) => match e {
            DdError::NotFound => return Err(DdError::ParentNotFound),
            DdError::NotDir => return Err(DdError::ParentNotDir),
            _ => return Err(e)
        }
    };
    if let Ok(_) = metadata(tx.clone(), path) {
        return Err(DdError::DirExists);
    }

    let inode = inode::alloc_inode(uid, true)?;
    let metadata = Metadata::new(inode.0, inode.1, tx.clone());

    // add parent/new
    dir_add_entry(parent_dd.inode_addr(), inode.0, &dir_name)?;

    // add new/.
    dir_add_entry(inode.0, inode.0, ".")?;

    // add new/..
    dir_add_entry(inode.0, parent_dd.inode, "..")?;

    // add into fd table
    let mut lock = utils::mutex_lock(fd_table.lock());
    lock.add_dir(inode.0, &inode.1).unwrap();

    logger::log(&format!("Create directory by user{uid}: {path}"));
    Ok(Dd::new(inode.0, metadata, tx.clone(), fd_table.clone()))
}

/// !!!NOTE!!!: will NOT remove sub-file or sub-dir!
/// 
/// ## Error
/// 
/// - InvalidPath
/// - NotFound
/// - NotDir
/// - DirOccupied
/// - DirIncorrupted
/// - ParentNotFound
/// - ParentNotDir
/// - IoErr
pub fn remove_dir(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str) -> Result<()> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::InvalidPath => return Err(DdError::InvalidPath),
            FsError::NotFound => return Err(DdError::NotFound),
            _ => panic!("{e:?}")
        }
    };
    let inode = inode::load_inode(inode_addr)?;
    let metadata = Metadata::new(inode_addr, inode, tx.clone());
    if !metadata.is_dir() {
        return Err(DdError::NotDir);
    }

    let mut lock = utils::mutex_lock(fd_table.lock());
    if let Ok(_) = lock.get_dir(inode_addr) {
        return Err(DdError::DirOccupied);
    }

    let path_vec: Vec<&str> = path.split('/').collect();
    let parent_path = path_vec[..path_vec.len()-1].join("/");
    let parent_dd = match open_dir(tx.clone(), fd_table.clone(), &parent_path) {
        Ok(d) => d,
        Err(e) => match e {
            DdError::NotFound => return Err(DdError::ParentNotFound),
            DdError::NotDir => return Err(DdError::ParentNotDir),
            _ => return Err(e)
        }
    };

    // remove entry from parent directory
    dir_remove_entry(parent_dd.inode_addr(), inode_addr)?;

    // remove file data
    let inode = inode::load_inode(inode_addr)?;
    let blocks = inode::get_blocks(&inode)?;
    data::free_blocks(&blocks)?;

    // free inode
    inode::free_inode(inode_addr)?;

    logger::log(&format!("Remove directory: {path}"));
    Ok(())
}

/// ## Error
/// 
/// - NotFound
/// - DirIncorrupted
/// - IoErr(e)
pub fn read_dir(dir_inode: u32) -> Result<Vec<Entry>> {
    let data = file::read_file(dir_inode)?;
    let size = data.len() / ENTRY_SIZE;
    let mut v = Vec::<Entry>::with_capacity(size);
    for i in 0..size {
        v.push(Entry::deserialize(&mut data[i*ENTRY_SIZE..(i+1)*ENTRY_SIZE].to_vec()).unwrap());
    }
    logger::log(&format!("Read directory: [dir_inode_addr] {dir_inode}"));
    Ok(v)
}

// [PASS]
/// ## Error
/// 
/// - NotFound
/// - DirIncorrupted
/// - NoEnoughSpace
/// - EntryExists
/// - IoErr
pub fn dir_add_entry(dir_inode: u32, entry_inode: u32, name: &str) -> Result<()> {
    let ents = read_dir(dir_inode)?;
    let ent = Entry { inode: entry_inode, name: String::from(name) };
    if ents.contains(&ent) {
        return Err(DdError::EntryExists)
    }
    let mut data = file::read_file(dir_inode)?;
    data.append(&mut ent.serialize());
    file::write_file(dir_inode, &data)?;
    logger::log(&format!("Add an entry to directory:\n    \
        [dir_inode_addr] {dir_inode}, \
        [entry_inode_addr] {entry_inode},\n    \
        [name] {name}\
    "));
    Ok(())
}

/// ## Error
/// 
/// - NotFound
/// - FileIncorrupted
/// - IoErr
pub fn dir_remove_entry(dir_inode: u32, entry_inode: u32) -> Result<()> {
    let mut v = read_dir(dir_inode)?;
    for (i, ent) in v.iter().enumerate() {
        if ent.inode == entry_inode {
            v = v.drain(i..i+1).collect();
            let mut data = Vec::<u8>::with_capacity(v.len() * ENTRY_SIZE);
            for ent in v {
                data.append(&mut ent.serialize());
            }
            file::write_file(dir_inode, &data)?;
            logger::log(&format!("Remove an entry from directory: \n    \
                [dir_inode_addr] {dir_inode}, \
                [entry_inode_addr] {entry_inode}\
            "));
            return Ok(())
        }
    }
    Err(DdError::NotFound)
}