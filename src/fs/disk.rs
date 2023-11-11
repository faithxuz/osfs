use crate::logger;
use crate::sedes::Serialize;
use super::superblock;

use std::fs::{self, File};
use std::io::{self, Seek, SeekFrom, Write, Read};

const DISK_PATH: & 'static str = "./the_disk";
pub const DISK_SIZE: u32 = 128 * 1024 * 1024;
pub const BLOCK_SIZE: u32 = 1024;
const BLOCK_COUNT: u32 = DISK_SIZE / BLOCK_SIZE;

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
    fn from(e: io::Error) -> Self {
        Self::IoErr(e)
    }
}

type Result<T> = result::Result<T, DiskError>;

// ====== FN ======

pub fn init_disk() -> Result<()> {
    match fs::metadata(DISK_PATH) {
        Ok(meta) => {
            let file_size = meta.len();
            if file_size < DISK_SIZE as u64 {
                logger::log("Insufficient file size. Remove original file.");
                fs::remove_file(DISK_PATH)?;
                create_disk()?;
            }
        },
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => {
                logger::log("Disk file not found.");
                create_disk()?;
            },
            _ => return Err(DiskError::IoErr(e))
        }
    };

    let mut f = File::open(DISK_PATH)?;
    let mut buf = [0u8; 1];
    f.read_exact(&mut buf);
    if buf[0] != 227 {
        logger::log("Incorrupted disk file. Remove original file.");
        fs::remove_file(DISK_PATH)?;
        create_disk()?;
    }

    logger::log("Initialized disk.");
    Ok(())
}

fn create_disk() -> Result<()> {
    let mut f = File::create(DISK_PATH)?;
    f.seek(SeekFrom::Start(DISK_SIZE as u64 + 1))?;
    f.write_all(b"\0")?;
    f.flush()?;
    logger::log("Created disk file.");

    // create superblock
    let buf = superblock::Superblock::new().serialize();
    write_blocks(&[(0, buf)].to_vec());

    // create dir: /
    /*
    unsafe {
        let mut inode_bm = (*disk.get()) .write_inode_bitmap().set_false(0);
    }
    let mut root = inode::alloc_inode(disk.clone(), 0, false).unwrap(); // user0 as root user
    root.update_blocks(0, &[0u32].to_vec());
    root.save();
    */

    Ok(())
}

pub fn read_blocks(addrs: &Vec<u32>) -> Result<Vec<u8>> {
    let mut v = Vec::<u8>::new();
    let mut f = File::open(DISK_PATH)?;
    for addr in addrs {
        if *addr <= 0 || *addr >= DISK_SIZE {
            return Err(DiskError::InvalidAddr);
        }
        let mut buf = [0u8; 1024];
        f.seek(SeekFrom::Start(*addr as u64 * BLOCK_SIZE as u64))?;
        f.read_exact(&mut buf)?;
        v.append(&mut buf.to_vec())
    }
    Ok(v)
}

pub fn write_blocks(data: &Vec<(u32, Vec<u8>)>) -> Result<()> {
    for (addr, _) in data {
        if *addr <= 0 || *addr >= DISK_SIZE {
            return Err(DiskError::InvalidAddr);
        }
    }
    let mut f = File::open(DISK_PATH)?;
    for (addr, buf) in data {
        f.seek(SeekFrom::Start(*addr as u64 * BLOCK_SIZE as u64))?;
        f.write_all(&buf[..])?;
    }
    f.flush()?;
    Ok(())
}