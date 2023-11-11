// ====== ERROR ====== 

use std::{error, fmt};

#[derive(Debug)]
pub enum InodeError {
    NoUsableBlock,
    InvalidAddr,
    DataTooBig,
    DiskErr(disk::DiskError),
    SystemTimeErr(std::time::SystemTimeError),
}

impl error::Error for InodeError {}

impl fmt::Display for InodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InodeError: {:?}", self)
    }
}

impl From<disk::DiskError> for InodeError {
    fn from(e: disk::DiskError) -> Self {
        Self::DiskErr(e)
    }
}

impl From<std::time::SystemTimeError> for InodeError {
    fn from(e: std::time::SystemTimeError) -> Self {
        Self::SystemTimeErr(e)
    }
}

type Result<T> = std::result::Result<T, InodeError>;

// ====== INODE ======

use super::utils;
use crate::sedes::{Serialize, Deserialize, SedesError};

pub const BITMAP_OFFSET: u32 = 1;
pub const INODE_OFFSET: u32 = BITMAP_OFFSET + 1;
const INODE_SIZE: usize = 64;
pub const INODE_COUNT: u32 = 4096;
const INODE_PER_BLOCK: u32 = super::disk::BLOCK_SIZE / INODE_SIZE as u32;
const MAX_BLOCKS: u32 = 8 + 256 + 256 * 256;
pub const MAX_SIZE: u32 = 1024 * MAX_BLOCKS;

pub const DIR_FLAG: u8 = 1 << 6;
pub const OWNER_RWX_FLAG: (u8, u8, u8) = (
    1 << 5, 1 << 4, 1 << 3
);
pub const OTHER_RWX_FLAG: (u8, u8, u8) = (
    1 << 2, 1 << 1, 1
);

#[derive(Debug, Default)]
pub struct Inode {
    pub uid: u8,                // 1
    pub mode: u8,               // 1
    pub size: u32,              // 4
    pub timestamp: u32,         // 4
    pub blocks: [u32; 8],       // 32
    pub indirect_block: u32,    // 4
    pub double_block: u32,      // 4
    // acquire 14 to 64
}

impl Serialize for Inode {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::<u8>::with_capacity(64);
        v.push(self.uid);
        v.push(self.mode);
        v.append(&mut utils::u32_to_u8arr(self.size).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.timestamp).to_vec());
        for i in 0..8 {
        v.append(&mut utils::u32_to_u8arr(self.blocks[i]).to_vec());
        }
        v.append(&mut utils::u32_to_u8arr(self.indirect_block).to_vec());
        v.append(&mut utils::u32_to_u8arr(self.double_block).to_vec());
        v
    }
}

impl Deserialize for Inode {
    fn deserialize(buf: &mut Vec<u8>) -> std::result::Result<Self, SedesError> {
        if buf.len() < INODE_SIZE {
            return Err(SedesError::DeserialBufferTooSmall);
        }
        let bytes = buf.as_slice();
        let mut me = Self::default();
        me.uid = u8::from_be(bytes[0]);
        me.mode = u8::from_be(bytes[1]);
        me.size = utils::u8arr_to_u32(&bytes[2..6]);
        me.timestamp = utils::u8arr_to_u32(&bytes[6..10]);
        for i in 0..8 {
            me.blocks[i] = utils::u8arr_to_u32(&bytes[10+4*i..10+4*(i+1)]);
        }
        me.indirect_block = utils::u8arr_to_u32(&bytes[42..46]);
        me.double_block = utils::u8arr_to_u32(&bytes[46..50]);
        Ok(me)
    }
}

// ====== FN ======

use super::disk;
use super::bitmap::BlockBitmap;

type Bitmap = BlockBitmap;

fn get_bitmap() -> Result<Bitmap> {
    let addrs: Vec<u32> = vec![BITMAP_OFFSET];
    let mut data = disk::read_blocks(&addrs)?;
    Ok(Bitmap::deserialize(&mut data).unwrap())
}

fn save_bitmap(bitmap: &Bitmap) -> Result<()> {
    let data = vec![(BITMAP_OFFSET, bitmap.serialize())];
    Ok(disk::write_blocks(&data)?)
}

pub fn alloc_inode(owner: u8, is_dir: bool) -> Result<Inode> {
    let mut bitmap = get_bitmap()?;
    bitmap.set_true(match bitmap.next_usable() {
        Some(p) => p,
        None => return Err(InodeError::NoUsableBlock)
    }).unwrap();
    let mut inode = Inode::default();
    inode.uid = owner;
    inode.mode = if is_dir { DIR_FLAG } else { 0 }
        + OWNER_RWX_FLAG.0 + OWNER_RWX_FLAG.1 + OTHER_RWX_FLAG.0;
    save_bitmap(&bitmap)?;
    Ok(inode)
}

pub fn free_inode(addr: u32) -> Result<()> {
    let mut bitmap = get_bitmap()?;
    match bitmap.set_false(addr) {
        Ok(_) => Ok(()),
        Err(e) => Err(InodeError::InvalidAddr)
    }
}

pub fn load_inode(addr: u32) -> Result<Inode> {
    let block = INODE_OFFSET + addr / INODE_PER_BLOCK;
    let pos = addr % INODE_PER_BLOCK * INODE_SIZE as u32;
    let buf = disk::read_blocks(&vec![block])?;
    Ok(match Inode::deserialize(&mut buf[
        pos as usize * INODE_SIZE
        ..(pos + 1) as usize * INODE_SIZE
    ].to_vec()) {
        Ok(i) => i,
        Err(e) => { todo!() }
    })

}

pub fn save_inode(addr: u32, inode: &Inode) -> Result<()> {
    let block = INODE_OFFSET + addr / INODE_PER_BLOCK;
    let pos = addr % INODE_PER_BLOCK * INODE_SIZE as u32;
    let mut buf = disk::read_blocks(&vec![block])?;
    let buf: Vec<_> = buf.splice(
        pos as usize * INODE_SIZE..(pos + 1) as usize * INODE_SIZE,
        inode.serialize()
    ).collect();
    disk::write_blocks(&vec![(INODE_OFFSET, buf.to_vec())])?;
    Ok(())
}