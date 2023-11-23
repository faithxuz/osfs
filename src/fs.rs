mod bitmap;
mod utils;
mod disk;

mod superblock;
mod inode;
mod data;

mod metadata;
mod file;
mod dir;

mod error {
    pub use super::bitmap::BitmapError;
    pub use super::disk::DiskError;
    pub use super::superblock::SuperblockError;
    pub use super::inode::InodeError;
    pub use super::data::DataError;
    pub use super::metadata::MetadataError;
    pub use super::file::FdError;
    pub use super::dir::DdError;
    pub use super::super::sedes::SedesError;
}

use error::*;

pub use superblock::Superblock;
pub use metadata::{Metadata, MetadataError, Rwx};
pub use file::{Fd, FdError};
pub use dir::{Dd, DdError, Entry as DirEntry};

// ====== ERROR ======

use std::{fmt, result, sync::mpsc};

#[derive(Debug)]
pub enum FsError {
    InnerError,
    InvalidPath,
    NotFound,
    NotFileButDir,
    NotDirButFile,
    Exists,
    MetadataErr(MetadataError),
    FileErr(FdError),
    DirErr(DdError),
    SendErr(String),
    RecvErr(String),
}

impl std::error::Error for FsError {}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[FS] {:?}", &self)
    }
}

impl From<MetadataError> for FsError {
    fn from(e: MetadataError) -> Self { Self::MetadataErr(e) }
}

impl From<FdError> for FsError {
    fn from(e: FdError) -> Self { Self::FileErr(e) }
}

impl From<DdError> for FsError {
    fn from(e: DdError) -> Self { Self::DirErr(e) }
}

impl From<mpsc::SendError<FsReq>> for FsError {
    fn from(e: mpsc::SendError<FsReq>) -> Self { Self::SendErr(format!("{e:?}")) }
}

impl From<mpsc::RecvError> for FsError {
    fn from(e: mpsc::RecvError) -> Self { Self::RecvErr(format!("{e:?}")) }
}

impl FsError {
    fn DiskErr(e: DiskError) -> Self {
        match e {
            DiskError::InvalidAddr => Self::InvalidPath,
            DiskError::IoErr(e) => {
                logger::log(&format!("[ERR][FS] IoErr: {e:?}"));
                Self::InnerError
            }
        }
    }
}

type Result<T> = result::Result<T, FsError>;

// ====== REQ & RES ======

/// **NO NEED TO KNOW DETAILS**
#[derive(Debug)]
pub enum FsReq {
    // fs request

    /// `tx`: send back result
    Superblock(Sender<Result<superblock::Superblock>>),

    /// `tx`: send back result
    /// 
    /// `path`: file/directory path
    Metadata(Sender<Result<metadata::Metadata>>, String),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    OpenFile(Sender<Result<Fd>>, String),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    /// 
    /// `uid`: creator (also owner) id
    CreateFile(Sender<Result<Fd>>, String, u8),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    RemoveFile(Sender<Result<()>>, String),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    OpenDir(Sender<Result<Dd>>, String),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    /// 
    /// `uid`: creator (also owner) id
    CreateDir(Sender<Result<Dd>>, String, u8),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    RemoveDir(Sender<Result<()>>, String),

    // Metadata request

    /// `tx`: send back result
    /// 
    /// `addr`: virtual address of inode
    /// 
    /// `inode`: the inode to write
    UpdateInode(Sender<Result<()>>, u32, inode::Inode),

    // Fd request

    /// `tx`: send back result
    /// 
    /// `file_inode`: inode virtual address of the file to read
    ReadFile(Sender<Result<Vec<u8>>>, u32),

    /// `tx`: send back result
    /// 
    /// `file_inode`: inode virtual address of the file to write
    /// 
    /// `data`: write content as u8 vector
    WriteFile(Sender<Result<()>>, u32, Vec<u8>),

    // Dd request

    /// `tx`: send back result
    /// 
    /// `dir_inode`: inode virtual address of the directory to read
    ReadDir(Sender<Result<Vec<DirEntry>>>, u32),

    /// `tx`: send back result
    /// 
    /// `dir_inode`: inode virtual address of the directory to modify
    /// 
    /// `entry_inode`: inode virtual address of the entry to add to directory
    /// 
    /// `name`: name of new entry
    DirAddEntry(Sender<Result<()>>, u32, u32, String),

    /// `tx`: send back result
    /// 
    /// `dir_inode`: inode virtual address of the directory to modify
    /// 
    /// `entry_inode`: inode virtual address of the entry to remove from directory
    DirRemoveEntry(Sender<Result<()>>, u32, u32),
}

// ====== FDTABLE ======

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Debug)]
pub struct FdTableEntry {
    pub count: u32,
    pub is_dir: bool,
}

#[derive(Debug)]
pub struct FdTable {
    data: HashMap<u32, FdTableEntry>,
}

impl FdTable {
    pub fn new() -> Self {
        Self { data: HashMap::<u32, FdTableEntry>::new() }
    }

    pub fn try_drop(&mut self, inode: u32) {
        if let Some(e) = self.data.get_mut(&inode) {
            if e.count == 1 {
                let _ = self.data.remove(&inode);
            } else {
                e.count -= 1; 
            }
        }
    }

    /// ## Error
    /// 
    /// - NotFileButDir
    fn add_file(&mut self, inode_addr: u32) {
        self.data.insert(inode_addr, FdTableEntry {
            count: 1, is_dir: false
        });
    }

    /// ## Error
    /// 
    /// - NotDirButFile
    fn add_dir(&mut self, inode_addr: u32) {
        self.data.insert(inode_addr, FdTableEntry{
            count: 1, is_dir: true
        });
    }

    pub fn check(&self, inode_addr: u32) -> Option<()> {
        match self.data.get(&inode_addr) {
            Some(_) => Some(()),
            None => None
        }
    }

    /// ## Error
    /// 
    /// - NotFileButDir
    pub fn get_file(&mut self, tx: Sender<FsReq>, inode_addr: u32, table_arc: Arc<Mutex<Self>>) -> Result<Fd> {
        let inode = match inode::load_inode(inode_addr) {
            Ok(i) => i,
            Err(e) => match e {
                error::InodeError::InvalidAddr => return Err(FsError::NotFound),
                error::InodeError::DiskErr(e) => return Err(FsError::DiskErr(e)),
                _ => panic!("{e:?}")
            }
        };
        let metadata = Metadata::new(inode_addr, inode, tx.clone());
        if metadata.is_dir() {
            return Err(FsError::NotFileButDir);
        }

        match self.data.get_mut(&inode_addr) {
            Some(ent) => {
                if !ent.is_dir {
                    return Err(FsError::NotFileButDir);
                }
                ent.count += 1;
            }
            None => self.add_file(inode_addr)
        }
        Ok(Fd::new(inode_addr, metadata, tx, table_arc))
    }

    /// ## Error
    /// 
    /// - NotDirButFile
    pub fn get_dir(&mut self, tx: Sender<FsReq>, inode_addr: u32, table_arc: Arc<Mutex<Self>>) -> Result<Dd> {
        let inode = match inode::load_inode(inode_addr) {
            Ok(i) => i,
            Err(e) => match e {
                error::InodeError::InvalidAddr => return Err(FsError::NotFound),
                error::InodeError::DiskErr(e) => return Err(FsError::DiskErr(e)),
                _ => panic!("{e:?}")
            }
        };
        let metadata = Metadata::new(inode_addr, inode, tx.clone());
        if !metadata.is_dir() {
            return Err(FsError::NotDirButFile);
        }

        match self.data.get_mut(&inode_addr) {
            Some(ent) => {
                if !ent.is_dir {
                    return Err(FsError::NotDirButFile);
                }
                ent.count += 1;
            }
            None => self.add_dir(inode_addr)
        }
        Ok(Dd::new(inode_addr, metadata, tx, table_arc))
    }
}

// ====== FN ======

use crate::logger;
use std::sync::mpsc::{Sender, Receiver};

pub fn start_fs(
    started: Sender<result::Result<(), & 'static str>>,
    self_tx: Sender<FsReq>,
    rx: Receiver<FsReq>,
) {
    if let Err(e) = disk::init_disk() {
        logger::log(&format!("[ERR][FS] Failed to initialize disk. Msg: {e}"));
        return
    }
    let fd_table = Arc::new(Mutex::new(FdTable::new()));
    logger::log("[FS] Created fd table");

    if let Err(e) = started.send(Ok(())) {
        logger::log(&format!("[ERR][FS] Failed to send start:ok. Msg: {e}"));
        return
    }

    for received in rx {
        let ds = format!("{:?}", &received);
        match received {
            FsReq::Superblock(tx) => {
                match superblock::superblock() {
                    Ok(sb) => tx_send(tx, Ok(sb), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::Metadata(tx, path) => {
                match metadata::metadata(self_tx.clone(), &path) {
                    Ok(m) => tx_send(tx, Ok(m), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::OpenFile(tx, path) => {
                match file::open_file(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(f) => tx_send(tx, Ok(f), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::CreateFile(tx, path, uid) => {
                match file::create_file(self_tx.clone(), fd_table.clone(), &path, uid) {
                    Ok(f) => tx_send(tx, Ok(f), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::RemoveFile(tx, path) => {
                match file::remove_file(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(_) => tx_send(tx, Ok(()), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::OpenDir(tx, path) => {
                match dir::open_dir(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(d) => tx_send(tx, Ok(d), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::CreateDir(tx, path, uid) => {
                match dir::create_dir(self_tx.clone(), fd_table.clone(), &path, uid) {
                    Ok(d) => tx_send(tx, Ok(d), &ds),
                    Err(e) => match e {
                        DdError::InvalidPath => tx_send(tx, Err(FsError::InvalidPath), &ds),
                        DdError::DirExists => tx_send(tx, Err(FsError::Exists), &ds),
                        _ => tx_send(tx, Err(FsError::InnerError), &ds)
                    }
                }
            },
            FsReq::RemoveDir(tx, path) => {
                match dir::remove_dir(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(_) => tx_send(tx, Ok(()), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::UpdateInode(tx, addr, inode) => {
                match inode::save_inode(addr, &inode) {
                    Ok(_) => tx_send(tx, Ok(()), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            }
            FsReq::ReadFile(tx, inode) => {
                match file::read_file(inode) {
                    Ok(v) => tx_send(tx, Ok(v), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::WriteFile(tx, inode, mut data ) => {
                match file::write_file(inode, &mut data) {
                    Ok(_) => tx_send(tx, Ok(()), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::ReadDir(tx, inode) => {
                match dir::read_dir(inode) {
                    Ok(v) => tx_send(tx, Ok(v), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::DirAddEntry(tx, dir_inode, entry_inode, name) => {
                match dir::dir_add_entry(dir_inode, entry_inode, &name) {
                    Ok(_) => tx_send(tx, Ok(()), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
            FsReq::DirRemoveEntry(tx, dir_inode, entry_inode) => {
                match dir::dir_remove_entry(dir_inode, entry_inode) {
                    Ok(_) => tx_send(tx, Ok(()), &ds),
                    Err(_) => tx_send(tx, Err(FsError::NotFound), &ds)
                }
            },
        }
    }
}

fn tx_send<T>(tx: Sender<Result<T>>, r: Result<T>, msg: &str) {
    if let Err(_) = tx.send(r) {
        logger::log(&format!("[ERR][FS] Sending failed! Request: {}", msg));
    }
}

// [PASS]
/// ## Error
/// 
/// - InvalidPath
/// - NotFound
fn path_to_inode(mut path: &str) -> Result<u32> {
    // assume path is absolute

    // root dir
    if path == "/" {
        return Ok(0);
    }

    // trim the '/' at the end
    if path.ends_with('/') {
        path = &path[..path.len()-1];
    }

    let mut path_vec: Vec<&str> = path.split('/').collect();
    if path_vec.len() < 1 {
        return Err(FsError::InvalidPath);
    }
    path_vec.drain(0..1);

    let mut inode = 0;
    for section in path_vec {
        if section == "" {
            return Err(FsError::InvalidPath);
        }
        let dir_now = match dir::read_dir(inode) {
            Ok(v) => v,
            Err(e) => match e {
                DdError::NotDir => return Err(FsError::NotFound),
                DdError::NotFound => return Err(FsError::NotFound),
                _ => panic!("{e:?}")
            }
        };
        let mut flag = false;
        for ent in dir_now {
            if ent.name == section {
                inode = ent.inode;
                flag = true;
                break
            }
        }
        if !flag {
            return Err(FsError::NotFound);
        }
    }
    Ok(inode)
}

/// Get superblock of the disk. Return [Superblock].
/// 
/// `fs_tx`: sender for sending request
pub fn superblock(fs_tx: &mut Sender<FsReq>) -> Result<Superblock> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::Superblock(tx))?;
    Ok(rx.recv()??)
}

/// Get metadata of a file or a directory. Return [Metadata].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file or directory
pub fn metadata(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<Metadata> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::Metadata(tx, String::from(path)))?;
    Ok(rx.recv()??)
}

/// Open a file. Return a file descriptor [Fd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file
pub fn open_file(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<Fd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::OpenFile(tx, String::from(path)))?;
    Ok(rx.recv()??)
}

/// Create a file. Return a file descriptor [Fd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file
/// 
/// `uid`: creator id
pub fn create_file(fs_tx: &mut Sender<FsReq>, path: &str, uid: u8) -> Result<Fd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::CreateFile(tx, String::from(path), uid))?;
    Ok(rx.recv()??)
}

/// Remove a file.
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file
pub fn remove_file(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::RemoveFile(tx, String::from(path)))?;
    Ok(rx.recv()??)
}

/// Open a directory. Return a directory descriptor [Dd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to directory
pub fn open_dir(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<Dd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::OpenDir(tx, String::from(path)))?;
    Ok(rx.recv()??)
}

/// Create a directory. Return a directory descriptor [Dd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to directory
/// 
/// `uid`: creator id
pub fn create_dir(fs_tx: &mut Sender<FsReq>, path: &str, uid: u8) -> Result<Dd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::CreateDir(tx, String::from(path), uid))?;
    Ok(rx.recv()??)
}

/// Remove a directory.
/// 
/// !!!NOTE!!!: will NOT remove sub-file or sub-dir!
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to directory
pub fn remove_dir(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::RemoveDir(tx, String::from(path)))?;
    Ok(rx.recv()??)
}