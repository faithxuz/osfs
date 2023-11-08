use crate::sedes::{Serialize, Deserialize};

// ====== ERROR ======

use std::{error, fmt};

#[derive(Debug)]
pub enum SuperblockError {
}

impl error::Error for SuperblockError {}

impl fmt::Display for SuperblockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataError! ErrorKind: {:?}", self)
    }
}

type Result<T> = std::result::Result<T, SuperblockError>;

// ====== SUPERBLOCK ======

pub struct Superblock {
}

impl Serialize for Superblock {
    fn serialize(&self) -> Vec<u8> {
        Vec::<u8>::new()
    }
}

impl Deserialize for Superblock {
    fn deserialize(buf: &Vec<u8>) -> Self {
        Self {}
    }
}

// ====== FN ======

pub fn init() -> Result<Superblock>  {
    Ok(Superblock {})
}