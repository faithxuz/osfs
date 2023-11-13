use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::{metadata, open_file};

const USAGE: &str = "Usage: cat [-nb] <file1> <file2> ...";
const PERMISSION: (bool, bool, bool) = (true, false, false);

pub fn cat(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    let mut opts = Options::new();
    opts.optflag("n", "", "Number all output lines");
    opts.optflag("b", "", "Number non-empty output lines");

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.free.is_empty() {
        return (ctx, String::from(USAGE));
    }

    let number_lines = matches.opt_present("n");
    let number_non_empty_lines = matches.opt_present("b");

    let mut return_str = String::new();

    for path in &matches.free {
        let file_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(e) => {
                return_str += &format!("Cannot convert '{}' to absolute path\n", path);
                continue;
            },
        };
        let meta = match metadata(&mut ctx.tx, &file_path) {
            Ok(m) => m,
            Err(e) => {
                return_str += &format!("Cannot find '{}'\n", path);
                continue;
            },
        };

        let rwx = permission::check_permission(ctx.uid, &meta, PERMISSION);
        if !rwx {
            return_str += &format!("Permission denied\n");
            continue;
        }

        if meta.is_dir() {
            return_str += &format!("'{}' is a directory", file_path)[..];
        }

        let mut file_fd = match open_file(&mut ctx.tx, &file_path) {
            Ok(fd) => fd,
            Err(e) => {
                return_str += &format!("Cannot open file: '{}'\n", path);
                continue;
            },
        };
        let file_vec = match file_fd.read() {
            Ok(v) => v,
            Err(e) => {
                return_str += &format!("Cannot read file: '{}'\n", path);
                continue;
            },
        };

        let lines = file_vec.split(|&c| c == b'\n');
        let mut output_vec = Vec::new();
        let mut line_num = 1;

        for line in lines {
            if number_lines || number_non_empty_lines && !line.is_empty() {
                output_vec.extend(format!("{:>6}\t", line_num).bytes());
                line_num += 1;
            }

            output_vec.extend(line);
            output_vec.push(b'\n');
        }
        return_str += &String::from_utf8_lossy(&output_vec).into_owned();
    }
    (ctx, return_str)
}