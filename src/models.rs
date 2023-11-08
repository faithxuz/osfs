mod disk;
mod superblock;
mod inode;
mod file;

pub use disk::{Disk, DiskError};
pub use superblock::{Superblock, SuperblockError};
pub use inode::InodeError;
pub use file::*;

use std::error;

pub fn init() -> Result<Disk, Box<dyn error::Error>> {
    let mut d = match disk::init() {
        Ok(d) => d,
        Err(e) => return Err(Box::new(e))
    };
    Ok(d)
}