use crate::sedes::{Serialize, Deserialize, SedesError};
use super::utils;
use super::{disk, data, inode};

const SUPERBLOCK_SIZE: usize = 29;

// ====== ERROR ======

use std::{error, fmt};

#[derive(Debug)]
pub enum SuperblockError {
    ReadErr,
    NotInitialized,
    DeserializeErr(& 'static str),
}

impl error::Error for SuperblockError {}

impl fmt::Display for SuperblockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataError: {:?}", self)
    }
}

type Result<T> = std::result::Result<T, SuperblockError>;

// ====== SUPERBLOCK ======

pub struct Superblock {
                                // 1
    inode_count: u32,           // 4
    inode_bitmap_offset: u32,   // 4
    data_bitmap_offset: u32,    // 4
    block_size: u32,            // 4
    inode_offset: u32,          // 4
    data_offset: u32,           // 4
    max_file_size: u32,         // 4
    magic: u8                   // 1
}

impl Superblock {
    pub fn new() -> Self {
        return Self {
            inode_count: inode::INODE_COUNT,
            inode_bitmap_offset: inode::BITMAP_OFFSET,
            data_bitmap_offset: data::BITMAP_OFFSET,
            block_size: disk::BLOCK_SIZE,
            inode_offset: inode::INODE_OFFSET,
            data_offset: data::DATA_OFFSET,
            max_file_size: inode::MAX_SIZE,
            magic: 172,
        }
    }
}

impl Serialize for Superblock {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::<u8>::new();
        v.push(227);
        v.append(&mut utils::u32_to_u8arr(self.inode_count).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.inode_bitmap_offset).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.data_bitmap_offset).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.block_size).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.inode_offset).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.data_offset).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.max_file_size).to_vec());
        v.push(self.magic);
        v
    }
}

impl Deserialize for Superblock {
    fn deserialize(buf: &mut Vec<u8>) -> std::result::Result<Self, SedesError> {
        if buf.len() < SUPERBLOCK_SIZE {
            return Err(SedesError::DeserialBufferTooSmall)
        }
        let bytes = buf.as_slice();
        let mut me = Self::new();
        me.inode_count = utils::u8arr_to_u32(&bytes[1..5]);
        me.inode_bitmap_offset = utils::u8arr_to_u32(&bytes[5..9]);
        me.data_bitmap_offset = utils::u8arr_to_u32(&bytes[9..13]);
        me.block_size = utils::u8arr_to_u32(&bytes[13..17]);
        me.inode_offset = utils::u8arr_to_u32(&bytes[17..21]);
        me.data_offset = utils::u8arr_to_u32(&bytes[21..25]);
        me.max_file_size = utils::u8arr_to_u32(&bytes[25..29]);
        me.magic = u8::from_be(bytes[29]);
        Ok(me)
    }
}