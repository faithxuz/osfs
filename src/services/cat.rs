// todo: metadata API; path analysis; uid parameter; Fd struct
// todo: panic!() => no panic

use getopts::Options;
use super::{Context, utils};
use crate::fs::open_file;

// pub fn cat<'a, 'b>(ctx: &'a mut Context, args: Vec<&'b str>) -> (&'a mut Context, String) {
pub fn cat(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from("Usage: cat [-nb] <file1> <file2> ..."));
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
        return (ctx, String::from("Usage: cat [-nb] <file1> <file2> ..."));
    }

    let number_lines = matches.opt_present("n");
    let number_non_empty_lines = matches.opt_present("b");

    let mut file_str = String::new();

    for mut file_path in &matches.free {
        file_path = match utils::convert_path_to_abs(&ctx.wd, file_path) {
            Ok(p) => &p,
            Err(e) => todo!()
        };
        let mut file_fd = match open_file(&mut ctx.tx, file_path) {
            Ok(fd) => fd,
            Err(e) => return (ctx, format!("Cannot open file: {}", file_path)),
        };
        let file_vec = match file_fd.read() {
            Ok(v) => v,
            Err(e) => return (ctx, format!("Cannot open file: {}", file_path)),
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
        file_str = String::from_utf8_lossy(&output_vec).into_owned();
    }
    (ctx, file_str)
}