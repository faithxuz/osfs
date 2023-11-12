use getopts::Options;
use super::{Context, utils};

pub fn cd(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from("Usage: cd <path>"));
    }

    let mut opts = Options::new();

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };
    
    if matches.free.len() > 1 {
        return (ctx, String::from("Too many arguments"));
    }

    let path = &matches.free[0]; 
    // is_dir: from where?
    // Where is my future?
    // Why am I alive?
    // I might as well die...
    let is_dir: bool = true;
    if is_dir {
        let dir_path = match utils::convert_path_to_abs(&ctx.wd, path) {
            Ok(p) => p,
            Err(e) => return (ctx, format!("Cannot convert path {} to absolute path", path)),
        };
        ctx.wd = dir_path;
        (ctx, String::new())
    } else {
        let file_path = match utils::convert_path_to_abs(&ctx.wd, path) {
            Ok(p) => p,
            Err(e) => return (ctx, format!("Cannot convert path {} to absolute path", path)),
        };
        ctx.wd = file_path;
        (ctx, String::new())
    }
}