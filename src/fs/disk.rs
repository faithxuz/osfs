// ====== ERROR ======

use std::{error, fmt, result};

#[derive(Debug)]
pub enum DiskError {
    InvalidAddr,
    IoErr(io::Error),
}

impl error::Error for DiskError {}

impl fmt::Display for DiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DiskError: {:?}", self)
    }
}

impl From<io::Error> for DiskError {
    fn from(e: io::Error) -> Self { Self::IoErr(e) }
}

type Result<T> = result::Result<T, DiskError>;

// ====== FN ======

use crate::logger;
use crate::sedes::Serialize;
use super::{superblock, inode, data, dir};

use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write, Read};

const DISK_PATH: & 'static str = "./the_disk";
pub const DISK_SIZE: u32 = 128 * 1024 * 1024;
pub const BLOCK_SIZE: u32 = 1024;

pub fn init_disk() -> Result<()> {
    match fs::metadata(DISK_PATH) {
        Ok(meta) => {
            let file_size = meta.len();
            if file_size < DISK_SIZE as u64 {
                logger::log("[FS] Insufficient file size. Remove original file.");
                fs::remove_file(DISK_PATH)?;
                create_disk()?;
            }
        },
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => {
                logger::log("[FS] Disk file not found.");
                create_disk()?;
            },
            _ => return Err(DiskError::IoErr(e))
        }
    };

    let mut buf = [0u8; 1];
    {
        let mut f = File::open(DISK_PATH)?;
        f.read_exact(&mut buf)?;
    }
    if buf[0] != 227 {
        logger::log("[FS] Found incorrupted disk file. Remove original file.");
        fs::remove_file(DISK_PATH)?;
        create_disk()?;
    }

    logger::log("[FS] Initialized disk.");
    logger::log(&format!("[FS] Superblock: {:?}", superblock::superblock().unwrap()));
    Ok(())
}

fn create_disk() -> Result<()> {
    {
        let mut f = OpenOptions::new().write(true).create(true).truncate(false).open(DISK_PATH)?;
        f.seek(SeekFrom::Start(DISK_SIZE as u64 + 1))?;
        f.write_all(&[1,0])?;
        f.flush()?;
    }
    logger::log("[FS] Created disk file.");

    // create superblock
    let buf = superblock::Superblock::new().serialize();
    write_blocks(&[(0, buf)].to_vec())?;
    logger::log("[FS] Initialized superblock.");

    // initialize inode and data bitmap
    let buf = [0u8; BLOCK_SIZE as usize];
    write_blocks(&[(inode::BITMAP_OFFSET, buf.to_vec())].to_vec())?;
    let mut data = Vec::new();
    for addr in data::BITMAP_OFFSET..data::DATA_OFFSET {
        data.push((addr, buf.to_vec()));
    }
    write_blocks(&data)?;

    // create dir: /
    let (root_inode_addr, _) = match inode::alloc_inode(0, true) {
        Ok(a) => a,
        Err(e) => match e {
            super::inode::InodeError::DiskErr(e) => return Err(e),
            _ => panic!("{e:?}")
        }
    };
    if let Err(e) = dir::dir_add_entry(root_inode_addr, root_inode_addr, ".") {
        todo!()
    }

    logger::log("[FS] Initialized root dir.");

    Ok(())
}

// [PASS]
/// # Error
/// 
/// - InvalidAddr
/// - IoErr
pub fn read_blocks(addrs: &Vec<u32>) -> Result<Vec<u8>> {
    let mut v = Vec::<u8>::new();
    let mut f = File::open(DISK_PATH)?;
    for addr in addrs {
        if *addr >= DISK_SIZE {
            return Err(DiskError::InvalidAddr);
        }
        let mut buf = [0u8; 1024];
        f.seek(SeekFrom::Start((*addr * BLOCK_SIZE) as u64))?;
        f.read_exact(&mut buf)?;
        v.append(&mut buf.to_vec())
    }
    Ok(v)
}

// [PASS]
pub fn write_blocks(data: &Vec<(u32, Vec<u8>)>) -> Result<()> {
    for (addr, _) in data {
        if *addr >= DISK_SIZE {
            return Err(DiskError::InvalidAddr);
        }
    }
    let mut f = OpenOptions::new().write(true).create(true).truncate(false).open(DISK_PATH)?;
    for (addr, buf) in data {
        f.seek(SeekFrom::Start((*addr * BLOCK_SIZE) as u64))?;
        if buf.len() < BLOCK_SIZE as usize {
            f.write_all(&buf[..])?;
            f.write_all(b"\0")?;
        } else {
            f.write_all(&buf[..BLOCK_SIZE as usize])?;
        }
        f.flush()?;
    }
    f.seek(SeekFrom::Start(DISK_SIZE as u64 + 1))?;
    f.write_all(&[1,0])?;
    f.flush()?;
    Ok(())
}