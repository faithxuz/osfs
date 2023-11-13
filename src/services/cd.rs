use getopts::Options;
use crate::fs::metadata;
use super::{Context, utils, permission};

const USAGE: &str = "Usage: cd <directory>";
const PERMISSION: (bool, bool, bool) = (false, false, true);

pub fn cd(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    let mut opts = Options::new();

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };
    
    if matches.free.len() > 1 {
        return (ctx, String::from("Too many arguments\n"));
    }

    let path = &matches.free[0]; 
    let dir_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
        Ok(p) => p,
        Err(e) => return (ctx, format!("Cannot convert path '{}' to absolute path\n", path)),
    };
    let meta = match metadata(&mut ctx.tx, &dir_path) {
        Ok(m) => m,
        Err(e) => return (ctx, format!("Cannot find '{}'\n", path)),
    };

    let rwx = permission::check_permission(ctx.uid, &meta, PERMISSION);
    if !rwx {
        return (ctx, format!("Permission denied\n"));
    }

    if meta.is_dir() {
        ctx.wd = dir_path;
        (ctx, String::new())
    } else {
        return (ctx, format!("'{}' is not a directory\n", path));
    }
}