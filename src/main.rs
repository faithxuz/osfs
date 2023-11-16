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

use std::sync::mpsc;
use std::thread;
use simdisk::{
    start_fs,
    start_server,
    logger,
};

fn main() {
    logger::log("[MAIN] Simdisk starting...");
    let (fs_tx, fs_rx) = mpsc::channel();
    let (started_tx, started_rx) = mpsc::channel();
    let ft = fs_tx.clone();
    thread::spawn(|| start_fs(started_tx, ft, fs_rx));

    if let Err(e) = started_rx.recv().unwrap() {
        logger::log(e);
        return;
    }
    logger::log("[MAIN] Simdisk started.");

    start_server(fs_tx);
}