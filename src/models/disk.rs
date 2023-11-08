use crate::logger;
use std::fs::{self, File};
use std::io::{self, Seek, SeekFrom, Write, Read};

const DISK_PATH: & 'static str = "./the_disk";
pub const DISK_SIZE: u32 = 128 * 1024 * 1024;
pub const BLOCK_SIZE: u32 = 1024;
const BLOCK_COUNT: u32 = DISK_SIZE / BLOCK_SIZE;

// ====== ERROR ======

use std::{error, fmt};

#[derive(Debug)]
pub enum DiskError {
    DataBitmapNotfound,
    NoUsableBlock,
    InvalidAddr,
    IoErr(io::Error),
}

impl error::Error for DiskError {}

impl fmt::Display for DiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DiskError! Errorkind: {:?}", self)
    }
}

impl From<io::Error> for DiskError {
    fn from(e: io::Error) -> Self {
        Self::IoErr(e)
    }
}

type Result<T> = std::result::Result<T, DiskError>;

// ====== DISK ======

use super::inode;
use super::file;
use super::superblock;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct Disk {
    superblock: RwLock<superblock::Superblock>,
    data_bm: RwLock<file::DataBitmap>,
    inode_bm: RwLock<inode::InodeBitmap>,
}

impl Disk {
    pub fn read_inode_bitmap(&self) -> RwLockReadGuard<'_, file::DataBitmap> {
        match self.data_bm.read() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recover from RwLock poisoned: {poisoned:?}")[..]);
                l
            }
        }
    }

    pub fn write_inode_bitmap(&mut self) -> RwLockWriteGuard<'_, file::DataBitmap> {
        match self.data_bm.write() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recover from RwLock poisoned: {poisoned:?}")[..]);
                l
            }
        }
    }

    pub fn read_data_bitmap(&self) -> RwLockReadGuard<'_, inode::InodeBitmap> {
        match self.inode_bm.read() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recover from RwLock poisoned: {poisoned:?}")[..]);
                l
            }
        }
    }

    pub fn write_data_bitmap(&mut self) -> RwLockWriteGuard<'_, inode::InodeBitmap> {
        match self.inode_bm.write() {
            Ok(l) => l,
            Err(poisoned) => {
                let l = poisoned.into_inner();
                logger::log(&format!("Recover from RwLock poisoned: {poisoned:?}")[..]);
                l
            }
        }
    }

    pub fn read_block(&self, addr: u32) -> Result<Vec<u8>> {
        if addr <= 0 || addr >= DISK_SIZE {
            return Err(DiskError::InvalidAddr);
        }
        // check whether addr > disk
        let mut buf = [0u8; 1024];
        let mut f = File::open(DISK_PATH)?;
        f.seek(SeekFrom::Start((addr * BLOCK_SIZE) as u64))?;
        f.read_exact(&mut buf)?;
        Ok(buf.to_vec())
    }

    pub fn write_block(&mut self, addr: u32, data: &Vec<u8>) -> Result<()> {
        if addr <= 0 || addr >= DISK_SIZE {
            return Err(DiskError::InvalidAddr);
        }
        // check whether addr > disk
        let mut f = File::open(DISK_PATH)?;
        f.seek(SeekFrom::Start((addr * BLOCK_SIZE) as u64))?;
        f.write_all(&data[..])?;
        f.flush()?;
        Ok(())
    }
}

// ====== FN ======

pub fn init() -> Result<Disk> {
    // haven't implemented yet...
}

fn create_disk() -> Result<()> {
    let mut f = File::create(DISK_PATH)?;
    let mut ff = f.try_clone()?;
    f.seek(SeekFrom::Start(DISK_SIZE as u64 + 1))?;
    f.write_all(b"\0")?;
    f.flush()?;
    logger::log("Created disk file");
    init_disk()?;
    Ok(())
}

fn init_disk() -> Result<()> {
    // create superblock
    // create dir: /
    Ok(())
}