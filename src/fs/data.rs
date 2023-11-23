const BITMAP_BLOCK: u32 = 16;
pub const BITMAP_OFFSET: u32 = 1 + 1 + 256;
pub const DATA_OFFSET: u32 = BITMAP_OFFSET + BITMAP_BLOCK;

// ====== ERROR ======

use std::{error, fmt, result};
use super::error::*;

#[derive(Debug)]
pub enum DataError {
    InsufficientUsableBlocks,
    InvalidAddr,
    DiskErr(DiskError),
}

impl error::Error for DataError {}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DataError: {:?}", &self)
    }
}

impl From<DiskError> for DataError {
    fn from(e: DiskError) -> Self { Self::DiskErr(e) }
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
        Err(e) => return Err(DataError::DiskErr(e))
    };
    Ok(Bitmap::deserialize(&mut data).unwrap())
}

fn save_bitmap(bitmap: &Bitmap) -> Result<()> {
    let mut data = Vec::<(u32, Vec<u8>)>::with_capacity(BITMAP_BLOCK as usize);
    for i in 0..BITMAP_BLOCK {
        let bytes = match bitmap.get_serialized_map(i as usize) {
            Ok(b) => b,
            Err(_) => break
        };
        data.push((BITMAP_OFFSET + i, bytes));
    }
    match disk::write_blocks(&data) {
        Ok(_) => Ok(()),
        Err(e) => return Err(DataError::DiskErr(e))
    }
}

// [PASS]
/// ## Error
/// 
/// - InsufficientUsableBlocks
/// - DiskErr
pub fn alloc_blocks(count: u32) -> Result<Vec<u32>> {
    let mut bitmap = get_bitmap()?;
    if bitmap.rest_usable() < count {
        return Err(DataError::InsufficientUsableBlocks);
    }

    let mut v = Vec::<u32>::new();
    for _ in 0..count {
        let addr = bitmap.next_usable().unwrap();
        bitmap.set_true(addr).unwrap();
        v.push(addr + DATA_OFFSET);
    }
    save_bitmap(&bitmap)?;
    Ok(v)
}

// [PASS]
/// ## Error
/// 
/// - InvalidAddr
/// - DiskErr
pub fn free_blocks(addrs: &Vec<u32>) -> Result<()> {
    let mut bitmap = get_bitmap()?;
    for addr in addrs {
        if let Err(_) = bitmap.set_false(*addr - DATA_OFFSET) {
            return Err(DataError::InvalidAddr)
        };
    }
    save_bitmap(&bitmap)?;
    Ok(())
}