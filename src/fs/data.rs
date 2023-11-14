const BITMAP_SIZE: usize = 2048;
const BITMAP_BLOCK: u32 = 16;
pub const BITMAP_OFFSET: u32 = 1 + 1 + 256;
pub const DATA_OFFSET: u32 = BITMAP_OFFSET + BITMAP_BLOCK;
const MAX_DATA_BLOCK: u32 = 128 * 1024 - DATA_OFFSET;
const ADDR_OFFSET: u32 = DATA_OFFSET * disk::BLOCK_SIZE;

// ====== ERROR ======

use std::{error, fmt, result};

#[derive(Debug)]
pub enum DataError {
    InsufficientUsableBlocks,
    InvalidAddr,
}

impl error::Error for DataError {}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DataError: {:?}", &self)
    }
}

type Result<T> = result::Result<T, DataError>;

// ====== FN ======

use crate::sedes::Deserialize;

use super::disk;
use super::bitmap::Bitmap;

fn get_bitmap() -> Result<Bitmap> {
    let addrs: Vec<u32> = (BITMAP_OFFSET..DATA_OFFSET).collect();
    let mut data = match disk::read_blocks(&addrs) {
        Ok(d) => d,
        Err(e) => todo!()
    };
    Ok(Bitmap::deserialize(&mut data).unwrap())
}

fn save_bitmap(bitmap: &Bitmap) -> Result<()> {
    let mut data = Vec::<(u32, Vec<u8>)>::with_capacity(BITMAP_BLOCK as usize);
    for i in 0..BITMAP_BLOCK {
        let bytes = match bitmap.get_serialized_map(i as usize) {
            Ok(b) => b,
            Err(e) => break
        };
        data.push((BITMAP_OFFSET + i, bytes));
    }
    match disk::write_blocks(&data) {
        Ok(_) => Ok(()),
        Err(e) => todo!()
    }
}

pub fn alloc_blocks(count: u32) -> Result<Vec<u32>> {
    let mut bitmap = get_bitmap()?;
    let mut v = Vec::<u32>::new();
    for _ in 0..count {
        let addr = match bitmap.next_usable() {
            Some(a) => a,
            None => return Err(DataError::InsufficientUsableBlocks)
        };
        if let Err(e) = bitmap.set_true(addr) {
            return Err(DataError::InvalidAddr)
        };
        v.push(addr + ADDR_OFFSET);
    }
    save_bitmap(&bitmap)?;
    Ok(v)
}

pub fn free_blocks(addrs: &Vec<u32>) -> Result<()> {
    let mut bitmap = get_bitmap()?;
    for addr in addrs {
        if let Err(e) = bitmap.set_false(*addr - ADDR_OFFSET) {
            return Err(DataError::InvalidAddr)
        };
    }
    save_bitmap(&bitmap)?;
    Ok(())
}