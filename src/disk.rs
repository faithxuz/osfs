use crate::SdResult;
use std::fs::{self, File};
use std::io::{ErrorKind, Seek, SeekFrom, Write};

static DISK_PATH: & 'static str = "./the_disk";
static DISK_SIZE: u64 = 128 * 1024 * 1024;

pub fn check_disk() -> SdResult<()> {
    let file_size = match fs::metadata(DISK_PATH) {
        Ok(meta) => meta.len(),
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                eprintln!("Disk file not found.");
                create_disk()
            },
            _ => panic!("{e:?}")
        }
    };
    if file_size < DISK_SIZE {
        eprintln!("Size of disk file insufficient. Remove disk file.");
        fs::remove_file(DISK_PATH).unwrap();
        create_disk();
    }
    // if disk not initialized
    // init_disk()?;
    Ok(())
}

pub fn create_disk() -> u64 {
    let mut f = File::create(DISK_PATH).unwrap();
    f.seek(SeekFrom::Start(DISK_SIZE + 1)).unwrap();
    f.write_all(b"\0").unwrap();
    f.flush().unwrap();
    eprintln!("Created disk file");
    f.metadata().unwrap().len()
}

fn init_disk() -> SdResult<()> {
    // create superblock
    // create dir: /
    Ok(())
}