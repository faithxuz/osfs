use crate::{utils, SdResult};
use crate::sedes::{Serialize, Deserialize};
use std::{error, fmt};

// ====== ERROR ======

#[derive(Debug)]
pub enum ErrorKind {
    BitmapNotfound,
    NoUsable,
}

#[derive(Debug)]
pub struct DataError {
    kind: ErrorKind,
}

impl DataError {
    pub fn new(kind: ErrorKind) -> Box<Self> {
        Box::new(Self { kind })
    }
}

impl error::Error for DataError {}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataError! ErrorKind: {:?}", self.kind)
    }
}

// ====== BITMAP ======

use crate::bitmap::Bitmap;

pub const BLOCK_SIZE: u32 = 1024;
const BITMAP_SIZE: usize = 2048;
const BITMAP_BLOCK: u32 = 16;
const BITMAP_OFFSET: u32 = 1 + 1 + 256;
const DATA_OFFSET: u32 = BITMAP_OFFSET + 16;
const MAX_DATA_BLOCK: u32 = 128 * 1024 - DATA_OFFSET;

struct DataBitmap {
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

impl Clone for DataBitmap {
    fn clone(&self) -> Self {
        Self { maps: self.maps.clone() }
    }
}

impl Copy for DataBitmap {}

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

// ====== FN ======

use crate::disk;
use std::sync::Mutex;
use std::io::{Seek, SeekFrom, Read, Write};

static BITMAP: Mutex<Option<DataBitmap>> = Mutex::new(None);

pub fn init() -> SdResult<()> {
    let mut buf = Vec::<u8>::new();
    for i in 0..BITMAP_BLOCK {
        let mut block = read_block(BITMAP_OFFSET + i)?;
        buf.append(&mut block);
    }
    *BITMAP.lock()? = Some(DataBitmap::deserialize(&buf));
    Ok(())
}

pub fn alloc_block() -> SdResult<u32> {
    let addr = match *BITMAP.lock()? {
        Some(mut b) => match b.next_usable() {
            Some(p) => p,
            None => return Err(DataError::new(ErrorKind::NoUsable))
        },
        None => return Err(DataError::new(ErrorKind::BitmapNotfound))
    };
    match *BITMAP.lock()? {
        Some(mut b) => b.set_true(addr),
        None => return Err(DataError::new(ErrorKind::BitmapNotfound))
    }
    Ok(addr + DATA_OFFSET)
}

pub fn free_block(addr: u32) -> SdResult<()> {
    match *BITMAP.lock()? {
        Some(mut b) => b.set_false(addr),
        None => return Err(DataError::new(ErrorKind::BitmapNotfound))
    }
    Ok(())
}

pub fn read_block(addr: u32) -> SdResult<Vec<u8>> {
    // check whether addr > disk
    let mut buf = [0u8; 1024];
    let mut f = disk::get_disk()?;
    f.seek(SeekFrom::Start((addr * BLOCK_SIZE) as u64))?;
    f.read_exact(&mut buf)?;
    Ok(buf.to_vec())
}

pub fn write_block(addr: u32, data: &Vec<u8>) -> SdResult<()> {
    // check whether addr > disk
    let mut f = disk::get_disk()?;
    f.seek(SeekFrom::Start((addr * BLOCK_SIZE) as u64))?;
    f.write_all(&data[..])?;
    f.flush()?;
    Ok(())
}