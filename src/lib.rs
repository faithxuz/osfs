pub mod logger;
mod sedes;
pub mod fs;
mod services;

const PORT: u16 = 7735;

use std::sync::mpsc;
use std::net::TcpStream;
use std::net::TcpListener;
use threadpool::ThreadPool;

pub fn start_server(fs_tx: mpsc::Sender<fs::FsReq>) {
    let listener = match TcpListener::bind(format!("127.0.0.1:{PORT}")) {
        Ok(l) => l,
        Err(e) => {
            logger::log(&format!("[ERR][SERVER] {}", &e));
            return
        }
    };
    let pool = ThreadPool::new(8);

    // for EVERY request, call fn handle in a new thread
    for s in listener.incoming() {
        let stream = match s {
            Ok(s) => s,
            Err(e) => {
                todo!()
            }
        };

        let tx = fs_tx.clone();
        pool.execute(move || handle(tx, stream))
    }
}
// run in seperated thread
pub fn handle(fs_tx: mpsc::Sender<fs::FsReq>, stream: TcpStream) {
    // extract stream to http
    // call the corresponding service
}