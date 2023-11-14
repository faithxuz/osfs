// [PASS]

use crate::sedes::{Serialize, Deserialize, SedesError};
use super::utils;

pub const BITMAP_SIZE: usize = 128;

// ====== ERROR ======

use std::{error, fmt, result};

use super::disk::BLOCK_SIZE;

#[derive(Debug)]
pub enum BitmapError {
    InvalidPos,
}

impl error::Error for BitmapError {}

impl fmt::Display for BitmapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BitmapError: {:?}", &self)
    }
}

type Result<T> = result::Result<T, BitmapError>;

// ====== BITMAP ======

pub struct BlockBitmap {
    data: [u64; BITMAP_SIZE]
}

impl BlockBitmap {
    fn get_u64(&self, pos: u32) -> Result<u64> {
        if (pos / 64) as usize > BITMAP_SIZE {
            return Err(BitmapError::InvalidPos);
        }
        Ok(self.data[(pos / 64) as usize])
    }

    fn set_u64(&mut self, pos: u32, map: u64) -> Result<()> {
        if (pos / 64) as usize > BITMAP_SIZE {
            return Err(BitmapError::InvalidPos);
        }
        self.data[(pos / 64) as usize] = map;
        Ok(())
    }

    pub fn check(&self, pos: u32) -> Result<bool> {
        let map = self.get_u64(pos)?;
        let flag: u64 = 1 << (pos % 64);
        Ok(map & flag > 0)
    }

    pub fn next_usable(&self) -> Option<u32> {
        for i in 0..BITMAP_SIZE {
            let mut flag = 1;
            for j in 0..64 {
                if self.data[i] & flag == 0 {
                    return Some((i*64 + j) as u32);
                }
                flag = flag << 1;
            }
        }
        None
    }

    pub fn rest_usable(&self) -> u32 {
        let mut result = 0;
        for m in self.data {
            result += 64 - utils::count_ones_in_u64(m);
        }
        result
    }

    pub fn set_true(&mut self, pos: u32) -> Result<()> {
        let map = self.get_u64(pos)?;
        let flag: u64 = 1 << (pos % 64);
        self.set_u64(pos, map | flag)?;
        Ok(())
    }

    pub fn set_false(&mut self, pos: u32) -> Result<()> {
        let map = self.get_u64(pos)?;
        let flag: u64 = !(1 << (pos % 64));
        self.set_u64(pos, map & !flag)?;
        Ok(())
    }
}

impl Serialize for BlockBitmap {
    fn serialize(&self) -> Vec<u8> {
        let mut v = Vec::<u8>::with_capacity(BITMAP_SIZE);
        for i in 0..BITMAP_SIZE {
            v.append(&mut utils::u64_to_u8arr(self.data[i]).to_vec());
        }
        v
    }
}

impl Deserialize for BlockBitmap {
    fn deserialize(buf: &mut Vec<u8>) -> std::result::Result<Self, SedesError> {
        // err handling...
        if buf.len() < BITMAP_SIZE * 8 {
            return Err(SedesError::DeserialBufferTooSmall)
        }
        let bytes = buf.as_slice();
        let mut me = Self { data: [0u64; BITMAP_SIZE] };
        for i in 0..BITMAP_SIZE {
            me.data[i] = utils::u8arr_to_u64(&bytes[8*i..8*(i+1)]);
        }
        Ok(me)
    }
}

pub struct Bitmap {
    maps: Vec<BlockBitmap>
}

impl Bitmap {
    fn get_pos(pos: u32) -> (u32, u32) {
        (pos / BITMAP_SIZE as u32, pos % BITMAP_SIZE as u32)
    }

    pub fn get_serialized_map(&self, index: usize) -> Result<Vec<u8>> {
        match self.maps.get(index) {
            Some(m) => Ok(m.serialize()),
            None => Err(BitmapError::InvalidPos)
        }
    }

    pub fn check(&self, pos: u32) -> Result<bool> {
        let (map, pos) = Self::get_pos(pos);
        match self.maps.get(map as usize) {
            Some(b) => b.check(pos),
            None => Err(BitmapError::InvalidPos)
        }
    }

    pub fn next_usable(&self) -> Option<u32> {
        for (i, map) in self.maps.iter().enumerate() {
            if let Some(p) = map.next_usable() {
                return Some((i * BITMAP_SIZE) as u32 + p);
            }
        }
        None
    }

    pub fn rest_usable(&self) -> u32 {
        let mut result = 0;
        for m in &self.maps {
            result += m.rest_usable();
        }
        result
    }

    pub fn set_true(&mut self, pos: u32) -> Result<()> {
        let (map, pos) = Self::get_pos(pos);
        match self.maps.get_mut(map as usize) {
            Some(b) => if let Err(e) = b.set_true(pos) {
                return Err(e)
            },
            None => return Err(BitmapError::InvalidPos)
        };
        Ok(())
    }

    pub fn set_false(&mut self, pos: u32) -> Result<()> {
        let (map, pos) = Self::get_pos(pos);
        match self.maps.get_mut(map as usize) {
            Some(b) => if let Err(e) = b.set_false(pos) {
                return Err(e)
            },
            None => return Err(BitmapError::InvalidPos)
        };
        Ok(())
    }
}

impl Deserialize for Bitmap {
    fn deserialize(buf: &mut Vec<u8>) -> result::Result<Self, SedesError> where Self: Sized {
        const BS: usize = BLOCK_SIZE as usize;
        let mut maps = Vec::<BlockBitmap>::new();
        if buf.len() % BS != 0 {
            buf.append(&mut [0u8; BS][..BS - buf.len() % BS].to_vec());
        }


        let bytes = &buf[..];
        for i in 0..(buf.len() / BS) {
            let m = BlockBitmap::deserialize(&mut bytes[i * BS..(i+1) * BS].to_vec()).unwrap();
            maps.push(m);
        }

        Ok(Self { maps })
    }
}