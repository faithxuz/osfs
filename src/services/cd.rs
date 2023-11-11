use crate::logger;

pub fn cd(args: Vec<&str>) -> () {
    if args.len() < 1 {
        logger::log("Usage: cd <path>");
    } else if args.len() < 2 {
        let path = args[0];
        // switch context
    } else {
        logger::log("Too many arguments");
    }
}