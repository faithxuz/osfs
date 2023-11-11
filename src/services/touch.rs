use crate::logger;
use crate::models::{create_file, FileError};
use super::Context;

pub fn touch(ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        logger::log("Usage: touch [-a] name");
    }
    
    if args.len() < 2 {
        let path = args[0];
        match create_file(ctx.uid, path) {
            Ok(()) => (),
            Err(fileError) => panic!(),
        }
    } else if args.len() < 3 {
        match args[0] {
            "-a" => {
                // update timestamp
            }
            _ => (),
        }
    } else {
        logger::log("Too many arguments");
    }
}