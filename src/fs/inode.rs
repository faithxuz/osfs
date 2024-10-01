// ====== ERROR ====== 

use std::{error, fmt, result};
use super::error::*;

#[derive(Debug)]
pub enum InodeError {
    NoUsableBlock,
    InvalidAddr,
    DataTooBig,
    DiskErr(DiskError),
}

impl error::Error for InodeError {}

impl fmt::Display for InodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InodeError: {:?}", self)
    }
}

impl From<DiskError> for InodeError {
    fn from(e: DiskError) -> Self { Self::DiskErr(e) }
}

type Result<T> = result::Result<T, InodeError>;

// ====== INODE ======

use super::utils;
use crate::sedes::{Serialize, Deserialize};
use chrono::prelude::*;

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

impl Inode {
    pub fn new(owner: u8, is_dir: bool) -> Self {
        let mut inode = Self::default();
        inode.uid = owner;
        if is_dir {
            inode.mode = DIR_FLAG
                + OWNER_RWX_FLAG.0 + OWNER_RWX_FLAG.1 + OWNER_RWX_FLAG.2
                + OTHER_RWX_FLAG.0 + OTHER_RWX_FLAG.2;
        } else {
            inode.mode = OWNER_RWX_FLAG.0 + OWNER_RWX_FLAG.1 + OTHER_RWX_FLAG.0;
        }
        inode.update_timestamp();
        inode
    }

    /// Update to now
    pub fn update_timestamp(&mut self) {
        let dt = Local::now();
        self.timestamp = dt.timestamp() as u32;
    }
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
        v.append(&mut [0u8; 14].to_vec());
        v
    }
}

impl Deserialize for Inode {
    fn deserialize(buf: &mut Vec<u8>) -> std::result::Result<Self, SedesError> {
        if buf.len() < INODE_SIZE {
            return Err(SedesError::DeserialBufferTooSmall);
        }
        let bytes = &buf[..];
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

impl Clone for Inode {
    fn clone(&self) -> Self {
        Self {
            uid: self.uid, mode: self.mode, size: self.size,
            timestamp: self.timestamp, blocks: self.blocks.clone(),
            indirect_block: self.indirect_block,
            double_block: self.double_block
        }
    }
}

impl Copy for Inode {}

// ====== FN ======

use super::{disk, data};
use super::bitmap::BlockBitmap;

type Bitmap = BlockBitmap;

// [PASS]
fn get_bitmap() -> Result<Bitmap> {
    let addrs: Vec<u32> = vec![BITMAP_OFFSET];
    let mut data = disk::read_blocks(&addrs)?;
    Ok(Bitmap::deserialize(&mut data).unwrap())
}

// [PASS]
fn save_bitmap(bitmap: &Bitmap) -> Result<()> {
    let data = vec![(BITMAP_OFFSET, bitmap.serialize())];
    Ok(disk::write_blocks(&data)?)
}

// [PASS]
/// ## Error
/// 
/// - NoUsableBlock
/// - DiskErr
pub fn alloc_inode(owner: u8, is_dir: bool) -> Result<(u32, Inode)> {
    let mut bitmap = get_bitmap()?;
    let addr = match bitmap.next_usable() {
        Some(p) => p,
        None => return Err(InodeError::NoUsableBlock)
    };
    bitmap.set_true(addr).unwrap();
    let inode = Inode::new(owner, is_dir);
    save_bitmap(&bitmap)?;
    save_inode(addr, &inode)?;
    Ok((addr, inode))
}

// [PASS]
/// ## Error
/// 
/// - InvalidAddr
/// - DiskErr
pub fn free_inode(addr: u32) -> Result<()> {
    let mut bitmap = get_bitmap()?;
    if let Err(_) = bitmap.set_false(addr) {
        return Err(InodeError::InvalidAddr);
    }
    save_bitmap(&bitmap)?;
    Ok(())
}

// [PASS]
/// ## Error
/// 
/// - InvalidAddr
/// - DiskErr
pub fn load_inode(addr: u32) -> Result<Inode> {
    if addr > INODE_COUNT {
        return Err(InodeError::InvalidAddr);
    }
    let block = INODE_OFFSET + addr / INODE_PER_BLOCK;
    let pos = addr % INODE_PER_BLOCK;
    let buf = disk::read_blocks(&vec![block])?;
    Ok(Inode::deserialize(&mut buf[
        pos as usize * INODE_SIZE
        ..(pos + 1) as usize * INODE_SIZE
    ].to_vec()).unwrap())

}

// [PASS]
/// ## Error
/// 
/// - DiskErr
pub fn save_inode(addr: u32, inode: &Inode) -> Result<()> {
    let block = INODE_OFFSET + addr / INODE_PER_BLOCK;
    let pos = addr % INODE_PER_BLOCK;
    let mut buf = disk::read_blocks(&vec![block])?;
    let s_inode = inode.serialize();
    buf.splice(
        pos as usize * INODE_SIZE..(pos + 1) as usize * INODE_SIZE,
        s_inode
    );
    disk::write_blocks(&vec![(block, buf.to_vec())])?;
    Ok(())
}

/// ## Error
/// 
/// - DiskErr
pub fn get_blocks(inode: &Inode) -> Result<Vec<u32>> {
    let mut v = Vec::<u32>::new();
    for addr in inode.blocks {
        if addr == 0 {
            return Ok(v)
        }
        v.push(addr);
    }

    // read indirect block
    match read_ind_block(inode.indirect_block) {
        Ok(mut u) => {
            v.append(&mut u.1);
            if u.0 {
                return Ok(v)
            }
        },
        Err(e) => return Err(e)
    }
    
    // read double indirect blocks
    match read_ind_block(inode.double_block) {
        Ok(u) => {
            for db in u.1 {
                if db == 0 {
                    return Ok(v)
                }
                match read_ind_block(db) {
                    Ok(mut u) => {
                        v.append(&mut u.1);
                        if u.0 {
                            return Ok(v)
                        }
                    },
                    Err(e) => return Err(e)
                }
            }
        },
        Err(e) => return Err(e)
    }

    Ok(v)
}

fn read_ind_block(addr: u32) -> Result<(bool, Vec<u32>)> {
    let buf = disk::read_blocks(&[addr].to_vec())?;
    const ADDR_COUNT: usize = disk::BLOCK_SIZE as usize / 4;
    let mut v = Vec::<u32>::with_capacity(ADDR_COUNT);
    for i in 0..ADDR_COUNT {
        let addr = utils::u8arr_to_u32(&buf[i*4..(i+1)*4]);
        if addr == 0 {
            return Ok((true, v))
        }
        v.push(addr);
    }
    Ok((false, v))
}

/// ## Error
/// 
/// - DataTooBig
/// - DiskErr
pub fn update_blocks(inode: &mut Inode, blocks: &Vec<u32>) -> Result<()> {
    if blocks.len() > MAX_BLOCKS as usize {
        return Err(InodeError::DataTooBig);
    }
    let mut it = blocks.iter();
    for i in 0..8 {
        let block = match it.next() {
            Some(b) => *b,
            None => return Ok(())
        };
        if block == 0 {
            return Ok(())
        }
        inode.blocks[i] = block;
    }

    // set indirect block
    if inode.indirect_block == 0 {
        inode.indirect_block = match data::alloc_blocks(1) {
            Ok(v) => *v.get(0).unwrap(),
            Err(e) => match e {
                DataError::InsufficientUsableBlocks => return Err(InodeError::NoUsableBlock),
                DataError::DiskErr(e) => return Err(InodeError::DiskErr(e)),
                _ => panic!("{e:?}")
            }
        };
    }
    let mut buf = Vec::<u8>::with_capacity(disk::BLOCK_SIZE as usize);
    for _ in 0..INODE_PER_BLOCK {
        let block = match it.next() {
            Some(b) => *b,
            None => {
                disk::write_blocks(&[(inode.indirect_block, buf)].to_vec())?;
                return Ok(())
            }
        };
        if block == 0 {
            disk::write_blocks(&[(inode.indirect_block, buf)].to_vec())?;
            return Ok(())
        }
        buf.append(&mut utils::u32_to_u8arr(block).to_vec());
    }
    disk::write_blocks(&[(inode.indirect_block, buf)].to_vec())?;

    // set double indirect block
    if inode.double_block == 0 {
        inode.double_block = match data::alloc_blocks(1) {
            Ok(v) => *v.get(0).unwrap(),
            Err(e) => match e {
                DataError::InsufficientUsableBlocks => return Err(InodeError::NoUsableBlock),
                DataError::DiskErr(e) => return Err(InodeError::DiskErr(e)),
                _ => panic!("{e:?}")
            }
        };
    }
    let buf = &disk::read_blocks(&[inode.double_block].to_vec())?;
    let addr_count = disk::BLOCK_SIZE as usize / 4;
    let mut double = Vec::<u32>::with_capacity(addr_count);
    for i in 0..addr_count {
        double.push(utils::u8arr_to_u32(&buf[i*4..(i+1)*4]));
    }

    let mut block_buf = Vec::<(u32, Vec<u8>)>::new();
    for i in 0..addr_count {
        let ind_block = double.get_mut(i).unwrap();
        if *ind_block == 0 {
            *ind_block = match data::alloc_blocks(1) {
                Ok(v) => *v.get(0).unwrap(),
                Err(e) => match e {
                    DataError::InsufficientUsableBlocks => return Err(InodeError::NoUsableBlock),
                    DataError::DiskErr(e) => return Err(InodeError::DiskErr(e)),
                    _ => panic!("{e:?}")
                }
            };
        }
        let mut buf = Vec::<u8>::with_capacity(disk::BLOCK_SIZE as usize);
        for _ in 0..INODE_PER_BLOCK {
            let block = match it.next() {
                Some(b) => *b,
                None => {
                    block_buf.push((*ind_block, buf));
                    let mut double_buf = Vec::<u8>::with_capacity(addr_count);
                    for addr in double {
                        double_buf.append(&mut utils::u32_to_u8arr(addr).to_vec());
                    }
                    block_buf.push((inode.double_block, double_buf));
                    disk::write_blocks(&block_buf)?;
                    return Ok(())
                }
            };
            if block == 0 {
                block_buf.push((*ind_block, buf));
                let mut double_buf = Vec::<u8>::with_capacity(addr_count);
                for addr in double {
                    double_buf.append(&mut utils::u32_to_u8arr(addr).to_vec());
                }
                block_buf.push((inode.double_block, double_buf));
                disk::write_blocks(&block_buf)?;
                return Ok(())
            }
            buf.append(&mut utils::u32_to_u8arr(block).to_vec());
        }
        block_buf.push((*ind_block, buf));
    }

    let mut double_buf = Vec::<u8>::with_capacity(addr_count);
    for addr in double {
        double_buf.append(&mut utils::u32_to_u8arr(addr).to_vec());
    }
    block_buf.push((inode.double_block, double_buf));
    disk::write_blocks(&block_buf)?;
    Ok(())
}