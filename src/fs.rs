mod utils;
mod bitmap;
mod disk;

mod superblock;
mod inode;
mod data;

mod metadata;
mod file;
mod dir;

// ====== ERROR ======

use std::{error, fmt, result};
use std::sync::mpsc::RecvError;

#[derive(Debug)]
pub enum FsError {
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
pub enum FsReq {
    // fs request

    /// `tx`: send back result
    /// 
    /// `path`: file path
    OpenFile(Sender<Result<file::Fd>>, String),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    /// 
    /// `uid`: creator (also owner) id
    CreateFile(Sender<Result<file::Fd>>, String, u8),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    RemoveFile(Sender<Result<()>>, String),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    OpenDir(Sender<Result<dir::Dd>>, String),

    /// `tx`: send back result
    /// 
    /// `path`: file path
    /// 
    /// `uid`: creator (also owner) id
    CreateDir(Sender<Result<dir::Dd>>, String, u8),

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
    ReadDir(Sender<Result<Vec<dir::Entry>>>, u32),

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
        match received {
            FsReq::OpenFile(tx, path) => {
                match handle_open_file(self_tx.clone(), &path) {
                    Ok(f) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::CreateFile(tx, path, uid) => {
                match handle_create_file(self_tx.clone(), &path, uid) {
                    Ok(f) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::RemoveFile(tx, path) => {
                match handle_remove_file(&path) {
                    Ok(_) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::OpenDir(tx, path) => {
                match handle_open_dir(self_tx.clone(), &path) {
                    Ok(d) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::CreateDir(tx, path, uid) => {
                match handle_create_dir(self_tx.clone(), &path, uid) {
                    Ok(d) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::RemoveDir(tx, path) => {
                match handle_remove_dir(&path) {
                    Ok(_) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::ReadFile(tx, inode) => {
                match handle_read_file(inode) {
                    Ok(v) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::WriteFile(tx, inode, data ) => {
                match handle_write_file(inode, &data) {
                    Ok(_) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::ReadDir(tx, inode) => {
                match handle_read_dir(inode) {
                    Ok(v) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::DirAddEntry(tx, dir_inode, entry_inode, name) => {
                match handle_dir_add_entry(dir_inode, entry_inode, &name) {
                    Ok(_) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
            FsReq::DirRemoveEntry(tx, dir_inode, entry_inode) => {
                match handle_dir_remove_entry(dir_inode, entry_inode) {
                    Ok(_) => { todo!() },
                    Err(e) => { todo!() }
                }
            },
        }
    }
}

/// Open a file. Return a file descriptor [file::Fd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file
pub fn open_file(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<file::Fd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::OpenFile(tx, String::from(path)));
    match rx.recv()? {
        Ok(f) => Ok(f),
        Err(e) => todo!()
    }
}

/// Create a file. Return a file descriptor [file::Fd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to file
/// 
/// `uid`: creator id
pub fn create_file(fs_tx: &mut Sender<FsReq>, path: &str, uid: u8) -> Result<file::Fd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::CreateFile(tx, String::from(path), uid));
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
    fs_tx.send(FsReq::RemoveFile(tx, String::from(path)));
    match rx.recv()? {
        Ok(_) => Ok(()),
        Err(e) => todo!()
    }
}

/// Open a directory. Return a directory descriptor [dir::Dd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to directory
pub fn open_dir(fs_tx: &mut Sender<FsReq>, path: &str) -> Result<dir::Dd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::OpenDir(tx, String::from(path)));
    match rx.recv()? {
        Ok(d) => Ok(d),
        Err(e) => todo!()
    }
}

/// Create a directory. Return a directory descriptor [dir::Dd].
/// 
/// `fs_tx`: sender for sending request
/// 
/// `path`: path to directory
/// 
/// `uid`: creator id
pub fn create_dir(fs_tx: &mut Sender<FsReq>, path: &str, uid: u8) -> Result<dir::Dd> {
    let (tx, rx) = mpsc::channel();
    fs_tx.send(FsReq::CreateDir(tx, String::from(path), uid));
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
    fs_tx.send(FsReq::RemoveDir(tx, String::from(path)));
    match rx.recv()? {
        Ok(_) => Ok(()),
        Err(e) => todo!()
    }
}

// ====== HANDLERS ======

fn handle_open_file(tx: Sender<FsReq>, path: &str) -> Result<file::Fd> {
    todo!()
}

fn handle_create_file(tx: Sender<FsReq>, path: &str, uid: u8) -> Result<file::Fd> {
    todo!()
}

fn handle_remove_file(path: &str) -> Result<()> {
    todo!()
}

fn handle_open_dir(tx: Sender<FsReq>, path: &str) -> Result<dir::Dd> {
    todo!()
}

fn handle_create_dir(tx: Sender<FsReq>, path: &str, uid: u8) -> Result<dir::Dd> {
    todo!()
}

fn handle_remove_dir(path: &str) -> Result<()> {
    todo!()
}

fn handle_read_file(inode: u32) -> Result<Vec<u8>> {
    todo!()
}

fn handle_write_file(inode: u32, data: &Vec<u8>) -> Result<()> {
    todo!()
}

fn handle_read_dir(dir_inode: u32) -> Result<Vec<dir::Entry>> {
    todo!()
}

fn handle_dir_add_entry(dir_inode: u32, entry_inode: u32, name: &str) -> Result<()> {
    todo!()
}

fn handle_dir_remove_entry(dir_inode: u32, entry_inode: u32) -> Result<()> {
    todo!()
}