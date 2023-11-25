 // [PASS]
 /*
 * iterate path in paths:
 *     if path exists
 *         update timestamp
 *     else if parent_path exists
 *         if path is a dir
 *             create_dir(path)
 *         else
 *             create_file(path)
 *     else
 *         return err
 */
use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::{metadata, create_file};

const USAGE: &str = "Usage: touch <file>...\n";
const PERMISSION: (bool, bool, bool) = (false, true, false);

pub fn touch(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    // define params
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

    if matches.free.is_empty() {
        return (ctx, String::from(USAGE));
    }

    let mut return_str = String::new();

    // iterate path in paths
    for path in &matches.free {
        let new_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(_) => {
                return_str += &format!("touch: Cannot convert '{}' to absolute path\n", path);
                continue;
            }
        };

        // update timestamp
        if let Ok(mut m) = metadata(&mut ctx.tx, &new_path) {
            if let Err(_) = m.update_timestamp() {
                return_str += &format!("touch: Cannot update timestamp: '{}'\n", path);
            }
            continue;
        }

        // split path
        let (parent_path, _) = utils::split_path(&new_path);

        match metadata(&mut ctx.tx, parent_path) {
            Ok(m) => {
                // check permission
                let rwx = permission::check_permission(ctx.uid, &m, PERMISSION);
                if !rwx {
                    return_str += &format!("touch: Permission denied: '{path}'\n");
                }

                // create file
                if let Err(_) = create_file(&mut ctx.tx, &new_path, ctx.uid) {
                    return_str += &format!("touch: Cannot create file: '{}'\n", path);
                }
            }
            Err(_) => {
                return_str += &format!("touch: cannot touch '{}': No such file or directory\n", path);
            }
        }
    }

    (ctx, return_str)
}