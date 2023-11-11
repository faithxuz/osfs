use std::path::Path;
use crate::logger;
use crate::models::{create_dir, metadata};

pub fn mkdir(args: Vec<&str>) -> () {
    // deal with the args to get parameters and the path
    if args.len() < 1 {
        logger::log("Usage: mkdir [-p] <path>");
    } else if args.len() < 2 {
        let metadata = metadata(args[0]).unwrap();
        match metadata.is_dir() {
            true => logger::log(&format!("Cannot create \" {} \": is a file", metadata.get_name())[..]),
            false => create_dir(uid, args[0]).unwrap(),
        }
    } else if args.len() < 3 {
        match args[1] {
            "-p" => {
                todo!()
            }
            _ => logger::log("Invalid argument"),
        }
    } else {
        logger::log("Too many arguments");
    }
}