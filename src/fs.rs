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

pub use metadata::{Metadata, MetadataError, Rwx};
pub use file::{Fd, FdError};
pub use dir::{Dd, DdError, Entry as DirEntry};

// ====== ERROR ======

use std::{fmt, result};
use std::sync::mpsc::RecvError;

#[derive(Debug)]
pub enum FsError {
    InvalidPath,
    NotFound,
    NotFileButDir,
    NotDirButFile,
    MetadataErr(MetadataError),
    FileErr(FdError),
    DirErr(DdError),
    RecvErr(RecvError),
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

impl From<RecvError> for FsError {
    fn from(e: RecvError) -> Self { Self::RecvErr(e) }
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
        if let Some(e) = self.data.get(&inode) {
            if e.count == 1 {
                let _ = self.data.remove(&inode);
            }
        }
    }

    /// ## Error
    /// 
    /// - NotFileButDir
    pub fn add_file(&mut self, inode_addr: u32, inode: &inode::Inode) -> Result<()> {
        if inode.mode & inode::DIR_FLAG > 0 {
            return Err(FsError::NotFileButDir)
        }
        self.data.insert(inode_addr, FdTableEntry {
            count: 0, is_dir: false
        });
        Ok(())
    }

    /// ## Error
    /// 
    /// - NotDirButFile
    pub fn add_dir(&mut self, inode_addr: u32, inode: &inode::Inode) -> Result<()> {
        if inode.mode & inode::DIR_FLAG == 0 {
            return Err(FsError::NotDirButFile)
        }
        self.data.insert(inode_addr, FdTableEntry{
            count: 0, is_dir: true
        });
        Ok(())
    }

    /// ## Error
    /// 
    /// - NotFileButDir
    pub fn get_file(&mut self, inode: u32) -> Result<Option<()>> {
        match self.data.get_mut(&inode) {
            Some(ent) => {
                if ent.is_dir {
                    return Err(FsError::NotFileButDir);
                }
                ent.count += 1;
                Ok(Some(()))
            },
            None => Ok(None)
        }
    }

    /// ## Error
    /// 
    /// - NotDirButFile
    pub fn get_dir(&mut self, inode: u32) -> Result<Option<()>> {
        match self.data.get_mut(&inode) {
            Some(ent) => {
                if !ent.is_dir {
                    return Err(FsError::NotFileButDir);
                }
                ent.count += 1;
                Ok(Some(()))
            },
            None => Ok(None)
        }
    }
}

// ====== FN ======

use crate::logger;
use std::sync::mpsc::{self, Sender, Receiver};

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
        let debug_str = format!("{:?}", &received);
        match received {
            FsReq::Superblock(tx) => {
                match superblock::superblock() {
                    Ok(sb) => {
                        if let Err(e) = tx.send(Ok(sb)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::Metadata(tx, path) => {
                match metadata::metadata(self_tx.clone(), &path) {
                    Ok(m) => {
                        if let Err(e) = tx.send(Ok(m)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::OpenFile(tx, path) => {
                match file::open_file(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(f) => {
                        if let Err(e) = tx.send(Ok(f)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::CreateFile(tx, path, uid) => {
                match file::create_file(self_tx.clone(), fd_table.clone(), &path, uid) {
                    Ok(f) => {
                        if let Err(e) = tx.send(Ok(f)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::RemoveFile(tx, path) => {
                match file::remove_file(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::OpenDir(tx, path) => {
                match dir::open_dir(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(d) => {
                        if let Err(e) = tx.send(Ok(d)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::CreateDir(tx, path, uid) => {
                match dir::create_dir(self_tx.clone(), fd_table.clone(), &path, uid) {
                    Ok(d) => {
                        if let Err(e) = tx.send(Ok(d)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::RemoveDir(tx, path) => {
                match dir::remove_dir(self_tx.clone(), fd_table.clone(), &path) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::UpdateInode(tx, addr, inode) => {
                match inode::save_inode(addr, &inode) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            }
            FsReq::ReadFile(tx, inode) => {
                match file::read_file(inode) {
                    Ok(v) => {
                        if let Err(e) = tx.send(Ok(v)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::WriteFile(tx, inode, data ) => {
                match file::write_file(inode, &data) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::ReadDir(tx, inode) => {
                match dir::read_dir(inode) {
                    Ok(v) => {
                        if let Err(e) = tx.send(Ok(v)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::DirAddEntry(tx, dir_inode, entry_inode, name) => {
                match dir::dir_add_entry(dir_inode, entry_inode, &name) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
            FsReq::DirRemoveEntry(tx, dir_inode, entry_inode) => {
                match dir::dir_remove_entry(dir_inode, entry_inode) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => todo!()
                }
            },
        }
    }
}

/// ## Error
/// 
/// - InvalidPath
/// - NotFound
fn path_to_inode(path: &str) -> Result<u32> {
    // assume path is absolute
    let mut path_vec: Vec<&str> = path.split('/').collect();
    if path_vec.len() < 1 {
        return Err(FsError::InvalidPath);
    }
    path_vec = path_vec.drain(0..1).collect();

    let mut inode = 0;
    let mut path = String::from("/");
    for section in path_vec {
        let dir_now = match dir::read_dir(inode) {
            Ok(v) => v,
            Err(e) => todo!()
        };
        for ent in dir_now {
            if ent.name == section {
                inode = ent.inode;
                path = path + section + "/";
                continue
            }
        }
        return Err(FsError::NotFound);
    }
    Ok(inode)
}

/// Get superblock of the disk. Return [Metadata].
/// 
/// `fs_tx`: sender for sending request
pub fn superblock(fs_tx: &mut Sender<FsReq>) -> Result<superblock::Superblock> {
    let (tx, rx) = mpsc::channel();
    if let Err(e) = fs_tx.send(FsReq::Superblock(tx)) {
        todo!()
    }
    match rx.recv()? {
        Ok(sb) => Ok(sb),
        Err(e) => todo!()
    }
}

/// Get metadata of a file or a directory. Return [Metadata].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file or directory
pub fn metadata(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<metadata::Metadata> {
    let (tx, rx) = mpsc::channel();
    if let Err(e) = fs_tx.send(FsReq::Metadata(tx, String::from(path))) {
        todo!()
    }
    match rx.recv()? {
        Ok(m) => Ok(m),
        Err(e) => todo!()
    }
}

/// Open a file. Return a file descriptor [Fd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file
pub fn open_file(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<Fd> {
    let (tx, rx) = mpsc::channel();
    if let Err(e) = fs_tx.send(FsReq::OpenFile(tx, String::from(path))) {
        todo!()
    }
    match rx.recv()? {
        Ok(f) => Ok(f),
        Err(e) => todo!()
    }
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
    if let Err(e) = fs_tx.send(FsReq::CreateFile(tx, String::from(path), uid)) {
        todo!()
    }
    match rx.recv()? {
        Ok(f) => Ok(f),
        Err(e) => todo!()
    }
}

/// Remove a file.
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file
pub fn remove_file(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    if let Err(e) = fs_tx.send(FsReq::RemoveFile(tx, String::from(path))) {
        todo!()
    }
    match rx.recv()? {
        Ok(_) => Ok(()),
        Err(e) => todo!()
    }
}

/// Open a directory. Return a directory descriptor [Dd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to directory
pub fn open_dir(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<Dd> {
    let (tx, rx) = mpsc::channel();
    if let Err(e) = fs_tx.send(FsReq::OpenDir(tx, String::from(path))) {
        todo!()
    }
    match rx.recv()? {
        Ok(d) => Ok(d),
        Err(e) => todo!()
    }
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
    if let Err(e) = fs_tx.send(FsReq::CreateDir(tx, String::from(path), uid)) {
        todo!()
    }
    match rx.recv()? {
        Ok(d) => Ok(d),
        Err(e) => todo!()
    }
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
    if let Err(e) = fs_tx.send(FsReq::RemoveDir(tx, String::from(path))) {
        todo!()
    }
    match rx.recv()? {
        Ok(_) => Ok(()),
        Err(e) => todo!()
    }
}