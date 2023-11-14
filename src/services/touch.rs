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
use crate::fs::{metadata, create_dir, create_file};

const USAGE: &str = "Usage: touch <name1> <name2> ...";
const PERMISSION: (bool, bool, bool) = (false, true, false);

// split parent path and sub path
fn split_path(path: &str) -> (&str, &str) {
    // split at the last '/' or '\'
    if let Some(index) = path.rfind('/') {
        let (parent_path, sub_path) = path.split_at(index);
        (&parent_path[..index], &sub_path[1..])
    } else if let Some(index) = path.rfind('\\') {
        let (parent_path, sub_path) = path.split_at(index);
        (&parent_path[..index], &sub_path[1..])
    } else {
        ("./\n", &path)
    }
}

pub fn touch(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    // define params: none
    let opts = Options::new();

    // parse args
    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.free.is_empty() {
        return (ctx, String::from(USAGE));
    }

    let mut return_str = String::new();

    // iterate path in paths
    for path in &matches.free {
        let new_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(e) => {
                return_str += &format!("Cannot convert '{}' to absolute path\n", path);
                continue;
            }
        };

        // update timestamp
        if let Ok(mut m) = metadata(&mut ctx.tx, &new_path) {
            if let Err(e) = m.update_timestamp() {
                return_str += &format!("Cannot update timestamp: '{}'\n", path);
            }
            continue;
        }

        // split path
        let (parent_path, sub_path) = split_path(&new_path);

        match metadata(&mut ctx.tx, &parent_path) {
            Ok(m) => {
                // check permission
                let rwx = permission::check_permission(ctx.uid, &m, PERMISSION);
                if !rwx {
                    return_str += &format!("Permission denied\n");
                }

                // create file
                if let Err(e) = create_file(&mut ctx.tx, &new_path, ctx.uid) {
                    return_str += &format!("Cannot create file: '{}'\n", path);
                }
            }
            Err(e) => {
                return_str += &format!("touch: cannot touch '{}': No such file or directory\n", path);
            }
        }
    }

    (ctx, return_str)
}