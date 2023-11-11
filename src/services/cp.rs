// only copy files for system path
// copy files and dirs for fs path
pub fn cp(args: Vec<&str>) -> () {
    let mut opts = Options::new();
    opts.optflag("r", "", "Copy a directory");
    opts.optflag("v", "", "Enable verbose output");

    let matches = match opts.parse(&args[0..]) {
        Ok(m) => m,
        Err(e) => {
            logger::log("Error: {}", e);
            return;
        }
    };

    if args.len() < 1 | matches.free.is_empty() {
        logger::log(&format!("Usage: cp [options] SOURSE DEST"));
        return;
    }

    let copy_dir = matches.opt_present("r");
    let verbose = matches.opt_present("v");

    let source = if !matches.free.is_empty() {
        Path::new(&matches.free[0])
    } else {
        eprintln!("Error: Missing source file");
        print_usage(&program, opts);
        return Ok(());
    };

    let dest = if matches.free.len() > 1 {
        Path::new(&matches.free[1])
    } else {
        eprintln!("Error: Missing destination file");
        print_usage(&program, opts);
        return Ok(());
    };

    let source_file = open_file()

}