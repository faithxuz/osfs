// todo: metadata API; path analysis; uid parameter; Fd struct
// todo: panic!() => no panic
use getopts::Options;

use super::Context;
use crate::logger;
use crate::models::create_file;

pub fn cat(ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return(ctx, String::from("Usage: cat [-nb] <file1> <file2> ..."));
    }

    let mut opts = Options::new();
    opts.optflag("n", "", "Number all output lines");
    opts.optflag("b", "", "Number non-empty output lines");

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return(ctx, f.to_string());
        }
    };

    if matches.free.is_empty() {
        return(ctx, String::from("Usage: cat [-nb] <file1> <file2> ..."));
    }

    let number_lines = matches.opt_present("n");
    let number_non_empty_lines = matches.opt_present("b");

    // open_file(?): Fd struct?
    for file_path in &matches.free {
        if let Ok(file_content) = std::fs::read_to_string(file_path) {
            let mut line_number = 1;

            for line in file_content.lines() {
                let mut output_line = String::new();

                if number_lines {
                    output_line.push_str(&format!("{:>6}\t", line_number));
                } else if number_non_empty_lines && !line.trim().is_empty() {
                    output_line.push_str(&format!("{:>6}\t", line_number));
                }

                output_line.push_str(line);

                logger::log(&format!("{}", output_line));
                line_number += 1;
            }
        } else {
            logger::log(&format!("Error reading file: {}", file_path));
        }
    }
}