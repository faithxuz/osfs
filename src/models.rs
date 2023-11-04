mod data;
pub mod superblock;
pub mod inode;
pub mod file;

use crate::SdResult;

pub fn init() -> SdResult<()> {
    data::init()?;
    superblock::init()?;
    inode::init()?;
    file::init()?;
    Ok(())
}