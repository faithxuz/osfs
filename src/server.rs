// ====== Req & Res ======
use serde;

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdReq {
    pub uid: u8,
    pub wd: String,
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct SdRes {
    pub wd: String,
    pub result: String,
}

// ====== FN ======

pub const PORT: u16 = 7735;

use crate::logger;
use super::{fs, services};
use std::io::{Write, BufReader, BufRead};
use std::sync::mpsc;
use std::net::{TcpStream, TcpListener};
use threadpool::ThreadPool;

type HandlerMap = std::collections::HashMap<
    String,
    fn (services::Context, Vec<&str>) -> (services::Context, String)
>;

pub fn start_server(fs_tx: mpsc::Sender<fs::FsReq>) {
    // init handler map
    let mut map = HandlerMap::new();
    map.insert(String::from("info"), services::info);
    map.insert(String::from("cd"), services::cd);
    map.insert(String::from("ls"), services::ls);
    map.insert(String::from("touch"), services::touch);
    map.insert(String::from("cat"), services::cat);
    map.insert(String::from("cp"), services::cp);
    map.insert(String::from("rm"), services::rm);
    map.insert(String::from("mkdir"), services::mkdir);
    map.insert(String::from("check"), services::check);

    // start tcp listening
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
        let m = map.clone();
        pool.execute(move || route(tx, stream, m))
    }
}

// run in seperated thread
pub fn route(fs_tx: mpsc::Sender<fs::FsReq>, mut stream: TcpStream, map: HandlerMap) {
    // extract stream as json: SdReq
    let mut req = String::new();
    let mut reader = BufReader::new(&mut stream);
    if let Err(e) = reader.read_line(&mut req) {
        todo!()
    }
    let req: SdReq = match serde_json::from_str(&req) {
        Ok(obj) => obj,
        Err(_) => {
            logger::log(&format!("[SERVER] Received unknown msg: {req}"));
            return;
        }
    };

    // call the corresponding service
    match map.get(&req.cmd) {
        Some(handler) => {
            logger::log(&format!(
                "[SERVER] From user{} received commad: {}\n    with args: {:?}",
                &req.uid, &req.cmd, &req.args
            ));
            let ctx = services::Context { uid: req.uid, wd: req.wd, tx: fs_tx.clone() };
            let args: Vec<&str> = req.args.iter().map(|s| s.as_str()).collect();
            let (ctx, s) = handler(ctx, args);

            // return result as json: SdRes
            let res = SdRes { wd: ctx.wd, result: s };
            let mut res_msg = serde_json::to_string(&res).unwrap();
            res_msg = res_msg + "\n";

            // send response
            if let Err(e) = stream.write_all(res_msg.as_bytes()) {
                todo!()
            }
            if let Err(e) = stream.flush() {
                todo!()
            }
        },
        None => {
            logger::log(&format!(
                "[SERVER] From user{} received unknown commad: {}\n    with args: {:?}",
                &req.uid, &req.cmd, &req.args
            ));
            // return unknown cmd error as json: SdRes
            let res = SdRes {
                wd: req.wd,
                result: format!("Unknown command: {}\n", req.cmd),
            };
            let mut res_msg = serde_json::to_string(&res).unwrap();
            res_msg = res_msg + "\n";

            // send response
            if let Err(e) = stream.write_all(res_msg.as_bytes()) {
                todo!()
            }
            if let Err(e) = stream.flush() {
                todo!()
            }
        }
    }
}