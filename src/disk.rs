use crate::{SdResult, logger};
use std::fs::{self, File};
use std::io::{ErrorKind, Seek, SeekFrom, Write, Read};

static DISK_PATH: & 'static str = "./the_disk";
static DISK_SIZE: u64 = 128 * 1024 * 1024;

pub fn check_disk() -> SdResult<()> {
    let file_size = match fs::metadata(DISK_PATH) {
        Ok(meta) => meta.len(),
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                logger::log("Disk file not found.");
                create_disk()?
            },
            _ => panic!("{e:?}")
        }
    };
    if file_size < DISK_SIZE {
        logger::log("Size of disk file insufficient. Remove disk file.");
        fs::remove_file(DISK_PATH).unwrap();
        create_disk();
    }
    let mut f = File::open(DISK_PATH)?;
    let mut buf = [0u8; 1];
    f.read_exact(&mut buf)?;
    if buf[0] == b'\0' {
        init_disk()?;
    }
    Ok(())
}

pub fn create_disk() -> SdResult<u64> {
    let mut f = File::create(DISK_PATH)?;
    f.seek(SeekFrom::Start(DISK_SIZE + 1))?;
    f.write_all(b"\0")?;
    f.flush()?;
    logger::log("Created disk file");
    Ok(f.metadata()?.len())
}

fn init_disk() -> SdResult<()> {
    // create superblock
    // create dir: /
    Ok(())
}

pub fn get_disk() -> SdResult<File> {
    Ok(File::create(DISK_PATH)?)
}