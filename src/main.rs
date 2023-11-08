/** Disk Struture
 * size: 128MB
 * block size: 1KB
 * block count: 128 * 1024
 * superblock: 1 blocks
 * inode bitmap: 1 blocks
 * inode size: 64B
 * inode count: 4096
 * inode: 256 blocks
 * data bitmap: 16 blocks
 */

use std::error::Error;
use std::sync::Arc;
use std::net::TcpListener;
use threadpool::ThreadPool;
use simdisk::{
    init,
    handle,
    logger::log,
    models::Disk,
};

const PORT: u16 = 7735;

fn main() {
    let mut disk = init().unwrap();
    log("Simdisk started...");
    server(disk).unwrap();
}

fn server(disk: Disk) -> Result<(), Box<dyn Error>>  {
    let mut d = Arc::new(disk);
    let listener = TcpListener::bind(format!("127.0.0.1:{PORT}"))?;
    let pool = ThreadPool::new(4);

    for s in listener.incoming() {
        let stream = match s {
            Ok(s) => s,
            Err(e) => return Err(Box::new(e))
        };

        pool.execute(move || handle(d.clone(), stream))
    }
    // for EVERY request, call fn handle in a new thread
    Ok(())
}