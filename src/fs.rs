mod bitmap;
mod utils;
mod disk;

mod superblock;
mod inode;
mod data;

mod metadata;
mod file;
mod dir;

pub use metadata::{Metadata, MetadataError};
pub use file::{Fd, FdError};
pub use dir::{Dd, DdError, Entry as DirEntry};

// ====== ERROR ======

use std::{error, fmt, result};
use std::sync::mpsc::RecvError;

#[derive(Debug)]
pub enum FsError {
    NotFound,
    NotFileButDir,
    NotDirButFile,
    RecvErr(RecvError),
}

impl error::Error for FsError {}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[FS] {:?}", &self)
    }
}

impl From<RecvError> for FsError {
    fn from(e: RecvError) -> Self {
        Self::RecvErr(e)
    }
}

type Result<T> = result::Result<T, FsError>;

// ====== REQ & RES ======

/// **NO NEED TO KNOW DETAILS**
#[derive(Debug)]
pub enum FsReq {
    // fs request

    /// `tx`: send back result
    /// 
    /// `path`: file/directory path
    Metadata(Sender<Result<metadata::Metadata>>, String),

    /// `tx`: send back result
    /// 
    /// `inode`: file/directory inode
    MetadataByInode(Sender<Result<metadata::Metadata>>, u32),

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

    // Fd and Dd request

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

    /// `tx`: send back result
    /// 
    /// `dir_inode`: inode virtual address of the directory to read
    ReadDir(Sender<Result<Vec<DirEntry>>>, u32),

    /// `tx`: send back result
    /// 
    /// `dir_inode`: inode virtual address of the directory to edit
    /// 
    /// `entry_inode`: inode virtual address of the entry to add to directory
    /// 
    /// `name`: name of new entry
    DirAddEntry(Sender<Result<()>>, u32, u32, String),

    /// `tx`: send back result
    /// 
    /// `dir_inode`: inode virtual address of the directory to edit
    /// 
    /// `entry_inode`: inode virtual address of the entry to remove from directory
    DirRemoveEntry(Sender<Result<()>>, u32, u32),
}

// ====== FDTABLE ======

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Debug)]
struct FdTable {
    data: HashMap<u32, (u32, bool, Arc<Mutex<inode::Inode>>)>,
}

impl FdTable {
    pub fn try_drop(&mut self, inode: u32) {
        if let Some(e) = self.data.get(&inode) {
            if e.0 == 1 {
                let _ = self.data.remove(&inode);
            }
        }
    }

    pub fn add_file(&mut self, inode_addr: u32, inode: inode::Inode) -> Result<()> {
        if inode.mode & inode::DIR_FLAG > 0 {
            return Err(FsError::NotFileButDir)
        }
        self.data.insert(inode_addr, (0, false, Arc::new(Mutex::new(inode))));
        Ok(())
    }

    pub fn add_dir(&mut self, inode_addr: u32, inode: inode::Inode) -> Result<()> {
        if inode.mode & inode::DIR_FLAG == 0 {
            return Err(FsError::NotDirButFile)
        }
        self.data.insert(inode_addr, (0, true, Arc::new(Mutex::new(inode))));
        Ok(())
    }

    pub fn get_file(&mut self, inode: u32) -> Result<Option<Arc<Mutex<inode::Inode>>>> {
        match self.data.get_mut(&inode) {
            Some(e) => {
                if (*e).1 {
                    return Err(FsError::NotFileButDir);
                }
                (*e).0 += 1;
                Ok(Some((*e).2.clone()))
            },
            None => Ok(None)
        }
    }

    pub fn get_dir(&mut self, inode: u32) -> Result<Option<Arc<Mutex<inode::Inode>>>> {
        match self.data.get_mut(&inode) {
            Some(e) => {
                if !(*e).1 {
                    return Err(FsError::NotFileButDir);
                }
                (*e).0 += 1;
                Ok(Some((*e).2.clone()))
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

    if let Err(e) = started.send(Ok(())) {
        logger::log(&format!("[ERR in FS] Failed to send start:ok. Msg: {e}"));
        return
    }
    for received in rx {
        let debug_str = format!("{:?}", &received);
        match received {
            FsReq::MetadataByInode(tx, inode) => {
                match metadata::handle_metadata_by_inode(self_tx.clone(), inode) {
                    Ok(m) => {
                        if let Err(e) = tx.send(Ok(m)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::Metadata(tx, path) => {
                match metadata::handle_metadata(self_tx.clone(), &path) {
                    Ok(m) => {
                        if let Err(e) = tx.send(Ok(m)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::OpenFile(tx, path) => {
                match file::handle_open_file(self_tx.clone(), &path) {
                    Ok(f) => {
                        if let Err(e) = tx.send(Ok(f)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::CreateFile(tx, path, uid) => {
                match file::handle_create_file(self_tx.clone(), &path, uid) {
                    Ok(f) => {
                        if let Err(e) = tx.send(Ok(f)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::RemoveFile(tx, path) => {
                match file::handle_remove_file(&path) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::OpenDir(tx, path) => {
                match dir::handle_open_dir(self_tx.clone(), &path) {
                    Ok(d) => {
                        if let Err(e) = tx.send(Ok(d)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::CreateDir(tx, path, uid) => {
                match dir::handle_create_dir(self_tx.clone(), &path, uid) {
                    Ok(d) => {
                        if let Err(e) = tx.send(Ok(d)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::RemoveDir(tx, path) => {
                match dir::handle_remove_dir(&path) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::ReadFile(tx, inode) => {
                match file::handle_read_file(inode) {
                    Ok(v) => {
                        if let Err(e) = tx.send(Ok(v)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::WriteFile(tx, inode, data ) => {
                match file::handle_write_file(inode, &data) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::ReadDir(tx, inode) => {
                match dir::handle_read_dir(inode) {
                    Ok(v) => {
                        if let Err(e) = tx.send(Ok(v)) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::DirAddEntry(tx, dir_inode, entry_inode, name) => {
                match dir::handle_dir_add_entry(dir_inode, entry_inode, &name) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
            FsReq::DirRemoveEntry(tx, dir_inode, entry_inode) => {
                match dir::handle_dir_remove_entry(dir_inode, entry_inode) {
                    Ok(_) => {
                        if let Err(e) = tx.send(Ok(())) {
                            logger::log(&format!("[ERR][FS] Sending failed! Request: {}", &debug_str));
                        };
                    },
                    Err(e) => { todo!() }
                }
            },
        }
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

/// Get metadata of a file or a directory. Return [Metadata].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `inode`: inode of file or directory
pub fn metadata_by_inode(fs_tx: &mut Sender<FsReq>, inode: u32) -> Result<metadata::Metadata> {
    let (tx, rx) = mpsc::channel();
    if let Err(e) = fs_tx.send(FsReq::MetadataByInode(tx, inode)) {
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