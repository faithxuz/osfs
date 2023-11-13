// ====== ERROR ======

use std::{error, fmt, result};

#[derive(Debug)]
pub enum DdError {
    InvalidPath,
    NotFound,
    NotDir,
    ParentNotFound,
    ParentNotDir,
    DirExists,
    DirOccupied,
    EntryExists,
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
use crate::sedes::{SedesError, Serialize, Deserialize};
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
        let mut name = (*&self.name[0..NAME_LEN].to_string()).clone().into_bytes();
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
        me.name = match String::from_utf8(buf[4..].to_vec()) {
            Ok(s) => s,
            Err(e) => todo!()
        };
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
        if let Err(e) = self.tx.send(FsReq::ReadDir(tx, self.inode)) {
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
                Err(e) => todo!()
            },
            Err(e) => todo!()
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
                Err(e) => todo!()
            },
            Err(e) => todo!()
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

use super::{inode, file, metadata::metadata};
use super::FdError;
use super::path_to_inode;

pub fn open_dir(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str) -> Result<Dd> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::NotFound => return Err(DdError::NotFound),
            FsError::NotDirButFile => return Err(DdError::NotDir),
            _ => todo!()
        }
    };
    let inode = match inode::load_inode(inode_addr) {
        Ok(i) => i,
        Err(e) => match e {
            inode::InodeError::InvalidAddr => return Err(DdError::NotFound),
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
    match lock.get_dir(inode_addr) {
        Ok(opt) => if let None = opt {
            if let Err(e) = lock.add_dir(inode_addr, &inode) {
                todo!()
            }
        },
        Err(e) => todo!()
    }

    logger::log(&format!("Open directory: {path}"));
    Ok(Dd::new(inode_addr, metadata, tx, fd_table.clone()))
}

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
            _ => todo!()
        }
    };
    if let Ok(_) = metadata(tx.clone(), path) {
        return Err(DdError::DirExists);
    }

    let inode = match inode::alloc_inode(uid, true) {
        Ok(i) => i,
        Err(e) => todo!()
    };
    let metadata = Metadata::new(inode.0, inode.1, tx.clone());

    // add parent/new
    if let Err(e) = dir_add_entry(parent_dd.inode_addr(), inode.0, &dir_name) {
        todo!()
    }
    // add new/.
    if let Err(e) = dir_add_entry(inode.0, inode.0, ".") {
        todo!()
    }
    // add new/..
    if let Err(e) = dir_add_entry(inode.0, parent_dd.inode, "..") {
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
    if let Err(e) = lock.add_dir(inode.0, &inode.1) {
        todo!()
    }

    logger::log(&format!("Create directory by user{uid}: {path}"));
    Ok(Dd::new(inode.0, metadata, tx.clone(), fd_table.clone()))
}

/// !!!NOTE!!!: will NOT remove sub-file or sub-dir!
/// 
/// ## Error
/// 
/// - NotFound
/// - NotDir
/// - DirOccupied
pub fn remove_dir(tx: Sender<FsReq>, fd_table: Arc<Mutex<FdTable>>, path: &str) -> Result<()> {
    let inode_addr = match path_to_inode(path) {
        Ok(i) => i,
        Err(e) => match e {
            FsError::NotFound => return Err(DdError::NotFound),
            FsError::NotDirButFile => return Err(DdError::NotDir),
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
        return Err(DdError::DirOccupied);
    }

    let path_vec: Vec<&str> = path.split('/').collect();
    let parent_path = path_vec[..path_vec.len()-1].join("/");
    let mut parent_dd = match open_dir(tx.clone(), fd_table.clone(), &parent_path) {
        Ok(d) => d,
        Err(e) => match e {
            DdError::NotFound => return Err(DdError::ParentNotFound),
            DdError::NotDir => return Err(DdError::ParentNotDir),
            _ => todo!()
        }
    };
    if let Err(e) = metadata(tx.clone(), path) {
        return Err(DdError::NotFound);
    }

    // remove entry from parent directory
    if let Err(e) = parent_dd.remove_entry(inode_addr) {
        todo!()
    };

    // remove directory file
    if let Err(e) = file::remove_file(tx.clone(), fd_table.clone(), path) {
        todo!()
    };

    logger::log(&format!("Remove directory: {path}"));
    Ok(())
}

pub fn read_dir(dir_inode: u32) -> Result<Vec<Entry>> {
    let data = match file::read_file(dir_inode) {
        Ok(d) => d,
        Err(e) => todo!()
    };
    let size = data.len() / ENTRY_SIZE;
    let mut v = Vec::<Entry>::with_capacity(size);
    for i in 0..size {
        v.push(match Entry::deserialize(&mut data[i*ENTRY_SIZE..(i+1)*ENTRY_SIZE].to_vec()) {
            Ok(ent) => ent,
            Err(e) => todo!()
        });
    }
    logger::log(&format!("Read directory: [dir_inode_addr]{dir_inode}"));
    Ok(v)
}

pub fn dir_add_entry(dir_inode: u32, entry_inode: u32, name: &str) -> Result<()> {
    let ents = match read_dir(dir_inode) {
        Ok(v) => v,
        Err(e) => todo!()
    };
    let ent = Entry { inode: entry_inode, name: String::from(name) };
    if ents.contains(&ent) {
        return Err(DdError::EntryExists)
    }
    let mut data = match file::read_file(dir_inode) {
        Ok(d) => d,
        Err(e) => todo!()
    };
    data.append(&mut ent.serialize());
    if let Err(e) =  file::write_file(dir_inode, &data) {
        todo!();
    };
    logger::log(&format!("Add an entry to directory: \
        [dir_inode_addr] {dir_inode}, 
        [entry_inode_addr] {entry_inode}, 
        [name] {name}
    "));
    Ok(())
}

pub fn dir_remove_entry(dir_inode: u32, entry_inode: u32) -> Result<()> {
    let mut v = match read_dir(dir_inode) {
        Ok(v) => v,
        Err(e) => todo!()
    };
    for (i, ent) in v.iter().enumerate() {
        if ent.inode == entry_inode {
            v = v.drain(i..i+1).collect();
            let mut data = Vec::<u8>::with_capacity(v.len() * ENTRY_SIZE);
            for ent in v {
                data.append(&mut ent.serialize());
            }
            if let Err(e) =  file::write_file(dir_inode, &data) {
                todo!();
            };
            logger::log(&format!("Remove an entry from directory: \
                [dir_inode_addr] {dir_inode}, 
                [entry_inode_addr] {entry_inode}
            "));
            return Ok(())
        }
    }
    Err(DdError::NotFound)
}