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
            name.append(&mut [0u8; NAME_LEN][self.name.len()..].to_vec());
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
        me.name = String::from(me.name.trim_end_matches('\0'));
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
            Err(e) => return Err(DdError::NotFound)
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
            Err(e) => return Err(DdError::NotFound)
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
            Err(e) => return Err(DdError::NotFound)
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

use super::{disk, inode, data, file, metadata::metadata};
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
    let dd = match lock.get_dir(tx.clone(), inode_addr, fd_table.clone()) {
        Ok(d) => d,
        Err(e) => match e {
            FsError::NotDirButFile => return Err(DdError::NotDir),
            _ => panic!("{e:?}")
        }
    };

    logger::log(&format!("[FS] Open directory: {path}"));
    Ok(dd)
}

/// ## Error
/// 
/// - InvalidPath
/// - ParentNotFound
/// - ParentNotDir
/// - DirExists
pub fn create_dir(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str, uid: u8) -> Result<Dd> {
    let mut path_vec: Vec<&str> = path.split('/').collect();
    path_vec.drain(0..1);
    let dir_name = String::from(match path_vec.pop() {
        Some(n) => n,
        None => return Err(DdError::InvalidPath)
    });
    let parent_path = String::from("/") + &path_vec.join("/");
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

    let mut inode = inode::alloc_inode(uid, true)?;

    // add parent/new
    dir_add_entry(parent_dd.inode_addr(), inode.0, &dir_name)?;

    // alloc data block
    let blocks = data::alloc_blocks(1)?;

    // write a empty entry
    let emp_ent = Entry { inode: 0, name: String::from("") };
    let data = [(
        *blocks.get(0).unwrap(),
        emp_ent.serialize()
    )].to_vec();
    disk::write_blocks(&data)?;

    // update and save inode
    inode::update_blocks(&mut inode.1, &blocks)?;
    inode::save_inode(inode.0, &inode.1)?;

    // add new/.
    dir_add_entry(inode.0, inode.0, ".")?;

    // add new/..
    dir_add_entry(inode.0, parent_dd.inode, "..")?;

    // add into fd table
    let mut lock = utils::mutex_lock(fd_table.lock());
    let dd = match lock.get_dir(tx.clone(), inode.0, fd_table.clone()) {
        Ok(d) => d,
        Err(e) => match e {
            FsError::NotDirButFile => return Err(DdError::NotDir),
            _ => panic!("{e:?}")
        }
    };

    logger::log(&format!("[FS] Create directory by user{uid}: {path}"));
    Ok(dd)
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

    {
        let lock = utils::mutex_lock(fd_table.lock());
        if let Some(_) = lock.check(inode_addr) {
            return Err(DdError::DirOccupied);
        }
    }

    let mut path_vec: Vec<&str> = path.split('/').collect();
    path_vec.drain(0..1);
    path_vec.pop();
    let parent_path = String::from("/") + &path_vec.join("/");
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
    let blocks = inode::get_blocks(&inode)?;
    data::free_blocks(&blocks)?;

    // free inode
    inode::free_inode(inode_addr)?;

    logger::log(&format!("[FS] Remove directory: {path}"));
    Ok(())
}

/// ## Error
/// 
/// - NotFound
/// - NotDir
/// - DirIncorrupted
/// - IoErr(e)
pub fn read_dir(dir_inode: u32) -> Result<Vec<Entry>> {
    let inode = inode::load_inode(dir_inode)?;
    if inode.mode & inode::DIR_FLAG == 0 {
        return Err(DdError::NotDir);
    }
    let blocks = inode::get_blocks(&inode)?;
    let data = disk::read_blocks(&blocks)?;

    let size = data.len() / ENTRY_SIZE;
    let mut v = Vec::<Entry>::with_capacity(size);
    for i in 0..size {
        let ent = Entry::deserialize(&mut data[i*ENTRY_SIZE..(i+1)*ENTRY_SIZE].to_vec()).unwrap();
        if ent.name == "" {
            break
        }
        v.push(ent);
    }

    Ok(v)
}

fn entries_to_data(ents: &Vec<Entry>) -> Vec<u8> {
    let mut data = Vec::<u8>::new();
    for ent in ents {
        data.append(&mut ent.serialize());
    }
    data
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
    let mut data = entries_to_data(&ents);
    data.append(&mut ent.serialize());
    let emp_ent = Entry { inode: 0, name: String::from("") };
    data.append(&mut emp_ent.serialize());
    file::write_file(dir_inode, &mut data)?;
    logger::log(&format!("[FS] Add an entry to directory:\n    \
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
            v.drain(i..i+1);
            let mut data = Vec::<u8>::with_capacity(v.len() * ENTRY_SIZE);
            for ent in v {
                data.append(&mut ent.serialize());
            }
            let emp_ent = Entry { inode: 0, name: String::from("") };
            data.append(&mut emp_ent.serialize());
            file::write_file(dir_inode, &mut data)?;
            logger::log(&format!("[FS] Remove an entry from directory: \n    \
                [dir_inode_addr] {dir_inode}, \
                [entry_inode_addr] {entry_inode}\
            "));
            return Ok(())
        }
    }
    Err(DdError::NotFound)
}