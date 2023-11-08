// ====== ERROR ====== 

use std::{error, fmt};
use super::DiskError;

#[derive(Debug)]
pub enum InodeError {
    NoUsable,
    TooBig,
    DiskErr(DiskError),
    SystemTimeError(time::SystemTimeError),
}

impl error::Error for InodeError {}

impl fmt::Display for InodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InodeError! ErrorKind: {:?}", self)
    }
}

impl From<time::SystemTimeError> for InodeError {
    fn from(e: time::SystemTimeError) -> Self {
        Self::SystemTimeError(e)
    }
}

impl From<DiskError> for InodeError {
    fn from(e: DiskError) -> Self {
        Self::DiskErr(e)
    }  
}

type Result<T> = std::result::Result<T, InodeError>;

// ====== INODE ======

use crate::utils;
use crate::sedes::{Serialize, Deserialize};
use std::time;
use std::sync::Arc;

const BITMAP_OFFSET: u32 = 1;
const INODE_OFFSET: u32 = BITMAP_OFFSET + 1;
const INODE_SIZE: usize = 64;
const INODE_COUNT: u32 = 4096;
const INODE_PER_BLOCK: u8 = (super::disk::BLOCK_SIZE / INODE_SIZE as u32) as u8;
const MAX_BLOCKS: u32 = 8 + 256 + 256 * 256;
const MAX_SIZE: u32 = 1024 * MAX_BLOCKS;

const DIR_FLAG: u8 = 1 << 6;
const OWNER_RWX_FLAG: (u8, u8, u8) = (
    1 << 5, 1 << 4, 1 << 3
);
const OTHER_RWX_FLAG: (u8, u8, u8) = (
    1 << 2, 1 << 1, 1
);

pub struct Rwx {
    read: bool,
    write: bool,
    execute: bool
}

struct InodeData {
    uid: u8,                // 1
    mode: u8,               // 1
    size: u32,              // 4
    timestamp: u32,         // 4
    blocks: [u32; 8],       // 32
    indirect_block: u32,    // 4
    double_block: u32,      // 4
    // acquire 14 to 64
}

impl InodeData {
    fn new() -> Self {
        Self {
            uid: 0, mode: 0, size: 0, timestamp: 0,
            blocks: [0; 8], indirect_block: 0, double_block: 0
        }
    }
}

pub struct Inode {
    addr: u32,
    disk: Arc<Disk>,
    data: InodeData
}

impl Inode {
    pub fn build(disk: Arc<Disk>, addr: u32, buf: &Vec<u8>) -> Self {
        Self {
            addr, disk,
            data: InodeData::deserialize(buf)
        }
    }

    pub fn build_new(disk: Arc<Disk>, addr: u32, owner: u8, is_dir: bool) -> Result<Self> {
        let mode = if is_dir { DIR_FLAG } else { 0 }
            + OWNER_RWX_FLAG.0 + OWNER_RWX_FLAG.1 + OTHER_RWX_FLAG.0;
        let mut ins = Self {
            addr, disk,
            data: InodeData::new()
        };
        ins.data.uid = owner;
        ins.data.mode = mode;
        ins.update_timestamp()?;
        Ok(ins)
    }

    pub fn addr(&self) -> u32 {
        self.addr
    }

    pub fn is_dir(&self) -> bool {
        self.data.mode & DIR_FLAG > 0
    }

    pub fn owner(&self) -> u8 {
        self.data.uid
    }

    pub fn owner_rwx(&self) -> Rwx {
        Rwx {
            read: self.data.mode & OWNER_RWX_FLAG.0 > 0,
            write: self.data.mode & OWNER_RWX_FLAG.1 > 0,
            execute: self.data.mode & OWNER_RWX_FLAG.2 > 0
        }
    }

    pub fn other_rwx(&self) -> Rwx {
        Rwx {
            read: self.data.mode & OTHER_RWX_FLAG.0 > 0,
            write: self.data.mode & OTHER_RWX_FLAG.1 > 0,
            execute: self.data.mode & OTHER_RWX_FLAG.2 > 0
        }
    }

    pub fn blocks(&self) -> Result<Vec<u32>> {
        let data = &self.data;
        let mut v = Vec::<u32>::new();
        for i in 0..8 {
            if data.blocks[i] == 0 {
                return Ok(v)
            }
            v.push(data.blocks[i]);
        }
        // indirect blocks
        // double indirect blocks
        Ok(v)
    }

    pub fn update_owner(&mut self, new_owner: u8) {
        self.data.uid = new_owner;
    }

    pub fn update_owner_rwx(&mut self, permission: &Rwx) {
        match permission.read {
            true => self.data.mode |= OWNER_RWX_FLAG.0,
            false => self.data.mode |= !OWNER_RWX_FLAG.0
        }
        match permission.write {
            true => self.data.mode |= OWNER_RWX_FLAG.1,
            false => self.data.mode |= !OWNER_RWX_FLAG.1
        }
        match permission.execute {
            true => self.data.mode |= OWNER_RWX_FLAG.2,
            false => self.data.mode |= !OWNER_RWX_FLAG.2
        }
    }

    pub fn update_other_rwx(&mut self, permission: &Rwx) {
        match permission.read {
            true => self.data.mode |= OTHER_RWX_FLAG.0,
            false => self.data.mode |= !OTHER_RWX_FLAG.0
        }
        match permission.write {
            true => self.data.mode |= OTHER_RWX_FLAG.1,
            false => self.data.mode |= !OTHER_RWX_FLAG.1
        }
        match permission.execute {
            true => self.data.mode |= OTHER_RWX_FLAG.2,
            false => self.data.mode |= !OTHER_RWX_FLAG.2
        }
    }

    pub fn update_timestamp(&mut self) -> Result<()> {
        self.data.timestamp = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)?
            .as_secs() as u32;
        Ok(())
    }

    pub fn update_blocks(&mut self, size: u32, addrs: &Vec<u32>) -> Result<()> {
        if size > MAX_SIZE || addrs.len() > MAX_BLOCKS as usize {
            return Err(InodeError::TooBig)
        }
        self.data.size = size;
        // ...
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let buf = self.data.serialize();
        // ...
        Ok(())
    }
}

impl Serialize for InodeData {
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

impl Deserialize for InodeData {
    fn deserialize(buf: &Vec<u8>) -> Self {
        if buf.len() < INODE_SIZE {
            panic!("too small!")
        }
        let bytes = buf.as_slice();
        let mut me = Self::new();
        me.uid = u8::from_be(bytes[0]);
        me.mode = u8::from_be(bytes[1]);
        me.size = utils::u8arr_to_u32(&bytes[2..6]);
        me.timestamp = utils::u8arr_to_u32(&bytes[6..10]);
        for i in 0..8 {
            me.blocks[i] = utils::u8arr_to_u32(&bytes[10+4*i..10+4*(i+1)]);
        }
        me.indirect_block = utils::u8arr_to_u32(&bytes[42..46]);
        me.double_block = utils::u8arr_to_u32(&bytes[46..50]);
        me
    }
}

// ====== BITMAP =======

use crate::bitmap::Bitmap;

const BITMAP_SIZE: usize = 64;
const BITMAP_BLOCK: u8 = 1;

pub struct InodeBitmap {
    maps: [u64; BITMAP_SIZE]
}

impl Bitmap for InodeBitmap {
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

impl Serialize for InodeBitmap {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::<u8>::with_capacity(BITMAP_SIZE);
        for i in 0..BITMAP_SIZE {
            v.append(&mut utils::u64_to_u8arr(self.maps[i]).to_vec());
        }
        v
    }
}

impl Deserialize for InodeBitmap {
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

// ====== FN ======

use super::Disk;

pub fn init_bitmap(d: &Disk) -> Result<InodeBitmap> {
    let buf = d.read_block(BITMAP_OFFSET)?;
    Ok(InodeBitmap::deserialize(&buf))
}

fn get_inode_pos(addr: u32) -> (u32, u32) {
    (
        (INODE_OFFSET + addr / INODE_PER_BLOCK as u32) as u32,
        (addr % INODE_PER_BLOCK as u32) * INODE_SIZE as u32
    )
}

pub fn alloc_inode(d: &mut Arc<Disk>, owner: u8, is_dir: bool) -> Result<Inode> {
    let mut lock = d.write_inode_bitmap();
    match lock.next_usable() {
            Some(p) => {
                lock.set_true(p);
                let mut inode = Inode::build_new(d.clone(), p, owner, is_dir)?;
                inode.save();
                Ok(inode)
            },
            None => Err(InodeError::NoUsable)
    }
}

pub fn free_inode(d: &mut Arc<Disk>, inode: &Inode) -> Result<()> {
    let mut lock = d.write_inode_bitmap();
    lock.set_false(inode.addr);
    Ok(())
}

pub fn load_inode(d: &Arc<Disk>, addr: u32) -> Result<Inode> {
    let (block, pos) = get_inode_pos(addr);
    let buf = d.read_block(block)?;
    Ok(Inode::build(d.clone(), addr, &buf[(pos * INODE_SIZE as u32) as usize..].to_vec()))
}