use crate::models::{remove_dir, remove_file, metadata, Metadata};
use crate::logger;

pub fn rm(args: Vec<&str>) -> () {
    if args.len() < 1 {
        logger::log("Usage: rm [-r] name");
    } else if args.len() < 2 {
        let metadata = match metadata(args[0]) {
            Ok(m) => m,
            Err(e) => panic!(),
        };
        // logger::log("Are you sure to delete file \"" + metadata.name() + "\" ?\n" +
        //             "yes:[y]/no:[n]");
        if metadata.is_dir() {
            logger::log("Cannot delete \" {metadata.name()} \": is a directory");
        } else {
            remove_file(args[0]);
        }
    } else if args.len() < 3 {
        let path = args[1];
        let metadata = metadata(path)?;
        let is_dir = metadata.is_dir();
        match args[0] {
            "-r" => {
                if is_dir {
                    remove_dir(path);
                } else {
                    logger::log(&format!("Are you sure to delete file {}", metadata.get_name()));
                    remove_file(path);
                }
            },
            _ => logger::log("Invalid argument"),
        }
    } else {
        logger::log("Too many arguments!");
    }
}