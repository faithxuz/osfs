// [PASS]

use getopts::Options;
use crate::fs::metadata;
use super::{Context, utils, permission};

const USAGE: &str = "Usage: cd <directory>\n";
const PERMISSION: (bool, bool, bool) = (false, false, true);

pub fn cd(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    // define params: none
    let mut opts = Options::new();
    opts.optflag("h", "", "Help");

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

    // support only one path
    if matches.free.len() > 1 {
        return (ctx, String::from("cd: Too many arguments\n"));
    }

    // get dir path
    let path = &matches.free[0]; 
    let dir_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
        Ok(p) => p,
        Err(e) => return (ctx, format!("cd: Cannot convert path '{}' to absolute path\n", path)),
    };
    let meta = match metadata(&mut ctx.tx, &dir_path) {
        Ok(m) => m,
        Err(e) => return (ctx, format!("cd: Cannot find '{}'\n", path)),
    };

    // check permission
    let rwx = permission::check_permission(ctx.uid, &meta, PERMISSION);
    if !rwx {
        return (ctx, format!("cd: Permission denied\n"));
    }

    // switch context
    if meta.is_dir() {
        ctx.wd = dir_path;
        (ctx, String::new())
    } else {
        return (ctx, format!("cd: '{}' is not a directory\n", path));
    }
}