use std::{error, fmt};

#[derive(Debug)]
pub enum SedesError {
    DeserialBufferTooSmall,
}

impl error::Error for SedesError {}

impl fmt::Display for SedesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[SEDES] {:?}", self)
    }
}

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

pub trait Deserialize {
    fn deserialize(buf: &mut Vec<u8>) -> Result<Self, SedesError> where Self: Sized;
}