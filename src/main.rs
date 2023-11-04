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

use std::net::TcpListener;
use threadpool::ThreadPool;
use simdisk::{
    SdResult,
    init,
    handle,
    logger::log
};

const PORT: u16 = 7735;

fn main() {
    init().unwrap();
    log("Simdisk started...");
    server().unwrap();
}

fn server() -> SdResult<()>  {
    let listener = TcpListener::bind(format!("127.0.0.1:{PORT}"))?;
    let pool = ThreadPool::new(4);

    for s in listener.incoming() {
        let stream = match s {
            Ok(s) => s,
            Err(e) => return Err(Box::new(e))
        };

        pool.execute(move || handle(stream))
    }
    // for EVERY request, call fn handle in a new thread
    Ok(())
}