 /*
 * iterate path in paths:
 *     if path doesn't exist
 *         return err
 *     (-l) return_str = list(ctx, path)
 *     if path is dir
 *         iterate entry in sub dir
 *             if path_append is start with '.' and -a is not specified
 *                 continue
 *             if entry is a dir
 *                 path_append += '/'
 *             add path_append to vec
 *     else
 *         if path_append is start with '.' and -a is not specified
 *             continue
 *         add path_append to vec
 */
use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::{metadata, open_file};

// define uasge and permission
const USAGE: &str = "Usage: cat [-nb] <file1> <file2> ...\n";
const PERMISSION: (bool, bool, bool) = (true, false, false);

pub fn cat(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    // define params
    let mut opts = Options::new();
    opts.optflag("h", "", "Help");
    opts.optflag("n", "", "Number all output lines");
    opts.optflag("b", "", "Number non-empty output lines");

    // parse args
    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.opt_present("h") {
        return (ctx, String::from(USAGE));
    }

    if matches.free.is_empty() {
        return (ctx, String::from(USAGE));
    }

    // convert parameters to bool variables
    let number_lines = matches.opt_present("n");
    let number_non_empty_lines = matches.opt_present("b");

    let mut return_str = String::new();

    // iterate path in paths
    for path in &matches.free {
        // check if exists
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

        // check permission
        let rwx = permission::check_permission(ctx.uid, &meta, PERMISSION);
        if !rwx {
            return_str += &format!("Permission denied\n");
            continue;
        }

        // open file
        if meta.is_dir() {
            return_str += &format!("'{}' is a directory\n", file_path)[..];
        }
        let mut file_fd = match open_file(&mut ctx.tx, &file_path) {
            Ok(fd) => fd,
            Err(e) => {
                return_str += &format!("Cannot open file: '{}'\n", path);
                continue;
            },
        };

        // get original file vector
        let file_vec = match file_fd.read() {
            Ok(v) => v,
            Err(e) => {
                return_str += &format!("Cannot read file: '{}'\n", path);
                continue;
            },
        };

        // split file vector with "\n"
        let lines = file_vec.split(|&c| c == b'\n');
        let mut output_vec = Vec::new();
        let mut line_num = 1;

        for line in lines {
            // "-n" -> input line number at the beginning
            // "-b" -> input line number at the beginning of non empty line
            if number_lines || number_non_empty_lines && !line.is_empty() {
                output_vec.extend(format!("{:>6}\t", line_num).bytes());
                line_num += 1;
            }

            // append output vector
            output_vec.extend(line);
            output_vec.push(b'\n');
        }
        // convert output vector to string
        return_str.push_str(&String::from_utf8_lossy(&output_vec));
        // return_str += &String::from_utf8_lossy(&output_vec).into_owned();
    }

    (ctx, return_str)
}