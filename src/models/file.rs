// just  interface
use crate::sedes::{Serialize, Deserialize};
use crate::utils;

// ====== Error ======

use std::{error, fmt};
use std::io;
use super::InodeError;

#[derive(Debug)]
pub enum FileError {
    NoUsable,
    InvalidAddr,
    FileNotfound,
    ReadErr,
    WriteErr,
    InodeErr(InodeError),
    IoErr(io::Error),
}

impl error::Error for FileError {}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileError! ErrorKind: {:?}", self)
    }
}

impl From<io::Error> for FileError {
    fn from(e: io::Error) -> Self {
        Self::IoErr(e)
    }
}

impl From<InodeError> for FileError {
    fn from(e: InodeError) -> Self {
        Self::InodeErr(e)
    }
}

type Result<T> = std::result::Result<T, FileError>;

// ====== METADATA ======

pub struct Metadata {
    name: String,
    permissions: u16,
    owner: u8,
    size: u64,
    is_dir: bool,
    /*
    created: SystemTime,
    last_accessed: SystemTime,
    last_modified: SystemTime,
    */
}

impl Metadata {
    pub fn build(name: &str, permissions: u16, owner: u8, size: u64, is_dir: bool) -> Self {
        Self {
            name: String::from(name), permissions, owner, size, is_dir
            // created: 0, last_accessed: 0, last_modified: 0
        }
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }
}

// ====== BITMAP ======

use crate::bitmap::Bitmap;

use super::disk::BLOCK_SIZE;
const BITMAP_SIZE: usize = 2048;
const BITMAP_BLOCK: u32 = 16;
const BITMAP_OFFSET: u32 = 1 + 1 + 256;
const DATA_OFFSET: u32 = BITMAP_OFFSET + 16;
const MAX_DATA_BLOCK: u32 = 128 * 1024 - DATA_OFFSET;

pub struct DataBitmap {
    maps: [u64; BITMAP_SIZE]
}

impl Bitmap for DataBitmap {
    fn get_map(&self, pos: u32) -> u64 {
        self.maps[(pos / 64) as usize]
    }

    fn set_map(&mut self, pos: u32, map: u64) {
        self.maps[(pos / 64) as usize] = map;
    }

    fn next_usable(&self) -> Option<u32> {
        for i in 0..BITMAP_SIZE {
            let mut flag = 1;
            for j in 0..64 {
                if self.maps[i] & flag > 0 {
                    return Some((i*64 + j) as u32);
                }
                flag << 1;
            }
        }
        None
    }
}

impl Serialize for DataBitmap {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::<u8>::with_capacity(BITMAP_SIZE);
        for i in 0..BITMAP_SIZE {
            v.append(&mut utils::u64_to_u8arr(self.maps[i]).to_vec());
        }
        v
    }
}

impl Deserialize for DataBitmap {
    fn deserialize(buf: &Vec<u8>) -> Self {
        // err handling...
        if buf.len() < BITMAP_SIZE * 8 {
            panic!("too small!")
        }
        let bytes = buf.as_slice();
        let mut me = Self { maps: [0u64; BITMAP_SIZE] };
        for i in 0..BITMAP_SIZE {
            me.maps[i] = utils::u8arr_to_u64(&bytes[8*i..8*(i+1)]);
        }
        me
    }
}

// ====== FD ======

pub struct Fd {
}

impl Fd {
    pub fn read(&self) -> Result<Vec<u8>> {
        Ok(Vec::<u8>::new())
    }

    pub fn write(&mut self, buf: &Vec<u8>) -> Result<()> {
        Ok(())
    }

    pub fn append(&mut self, buf: &Vec<u8>) -> Result<()> {
        Ok(())
    }
}

// ====== Dd ======

pub struct DirEntry {
    inode_addr: u32,
    name: String
}

pub struct Dd {
}

impl Dd {
    pub fn read(&self) -> Result<Vec<DirEntry>> {
        let v = Vec::<DirEntry>::new();
        Ok(v)
    }

    pub fn add(&mut self, name: &str) -> Result<()> {
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<()> {
        Ok(())
    }
}

// ====== FN ======

use std::sync::Arc;
use super::Disk;
use super::inode::{self, Inode, init_bitmap, InodeBitmap};

pub fn metadata(path: &str) -> Result<Metadata> {
    Ok(Metadata::build("", 0, 0, 0, false))
}

pub fn create_file(uid: u8, path: &str) -> Result<()> {
    Ok(())
}

pub fn open_file(path: &str) -> Result<Fd> {
    Ok(Fd {})
}

pub fn remove_file(path: &str) -> Result<()> {
    Ok(())
}

pub fn create_dir(uid: u8, path: &str) -> Result<()> {
    Ok(())
}

pub fn open_dir(path: &str) -> Result<Dd> {
    Ok(Dd {})
}

pub fn remove_dir(path: &str) -> Result<()> {
    Ok(())
}