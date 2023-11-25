 // [PASS]
 /*
 * iterate path in paths:
 *     if path doesn't exist
 *         return_str += err message
 *         continue
 *     if path is a file
 *         remove_file(path)
 *     else
 *         if -r is not specified
 *             continue
 *         remove_dir_recursively(path)            
 * 
 * ---fn remove_dir_recursively(dir_path) -> str
 *     iterate sub_entry in path Dd
 *         if sub_entry is ".." or "."
 *             continue
 *         if sub_path is a file
 *             remove_file(sub_path)
 *         else
 *             remove_dir_recursively(sub_path)
 *     remove_dir(dir_path)
 */
use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::{metadata, open_dir, remove_dir, remove_file};

// define uasge and permission
const USAGE: &str = "Usage: rm [-r] <file>...\n";
const PERMISSION: (bool, bool, bool) = (false, true, false);

fn remove_dir_recursively(ctx: &mut Context, dir_path: &str) -> String {
    let return_str = String::new();

    {
        // get sub entrys of dir
        let mut dir_dd = match open_dir(&mut ctx.tx, &dir_path) {
            Ok(m) => m,
            Err(_) => return format!("rm: Cannot find directory '{}'\n", dir_path),
        };
        let vec = match dir_dd.read() {
            Ok(v) => v,
            Err(_) => return format!("rm: Cannot read directory '{}'\n", dir_path),
        };

        // iterate entry in sub entrys
        for sub_entry in vec {
            // skip parent dir and itself
            if sub_entry.name == ".." || sub_entry.name == "." {
                continue;
            }

            // get sub path
            let parent_path = dir_path;
            let sub_path = match utils::convert_path_to_abs(&parent_path, &sub_entry.name) {
                Ok(p) => p,
                Err(_) => return format!("rm: Cannot convert '{}' to absolute path\n", sub_entry.name),
            };
            let sub_meta = match metadata(&mut ctx.tx, &sub_path) {
                Ok(m) => m,
                Err(_) => return format!("rm: Cannot find '{}'\n", sub_entry.name),
            };

            // check permission
            let rwx = permission::check_permission(ctx.uid, &sub_meta, PERMISSION);
            if !rwx {
                return format!("rm: Permission denied: '{sub_path}'\n");
            }

            // if sub meta is a dir
            if sub_meta.is_dir() {
                remove_dir_recursively(ctx, &sub_path);
            } else {
                // remove file
                if let Err(_) = remove_file(&mut ctx.tx, &sub_path) {
                    return format!("rm: Cannot remove file '{}'\n", sub_entry.name);
                }
            }
        }
    }

    // remove dir (itself)
    if let Err(_) = remove_dir(&mut ctx.tx, &dir_path) {
        return format!("rm: Cannot remove directory '{}'\n", dir_path);
    }

    return_str
}

pub fn rm(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    // define params
    let mut opts = Options::new();
    opts.optflag("h", "", "Help");
    opts.optflag("r", "", "Remove directories and their contents recursively");

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
    let remove_dir = matches.opt_present("r");

    let mut return_str = String::new();

    // iterate path in paths
    for path in matches.free {
        // open path
        let new_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(_) => {
                return_str += &format!("rm: Cannot convert '{}' to absolute path\n", path);
                continue;
            }
        };
        let meta = match metadata(&mut ctx.tx, &new_path) {
            Ok(m) => m,
            Err(_) => {
                return_str += &format!("rm: Cannot find '{}'\n", path);
                continue;
            },
        };

        // check permission
        let rwx = permission::check_permission(ctx.uid, &meta, PERMISSION);
        if !rwx {
            return_str += &format!("rm: Permission denied: '{path}'\n");
        }

        // check if path is a sir
        if meta.is_dir() {
            // remove dir recursively
            if remove_dir {
                return_str += &remove_dir_recursively(&mut ctx, &new_path);
            }
            else {
                return_str += &format!("rm: Cannot remove '{path}': Is a directory\n");
            }
        } else {
            // remove file
            if let Err(_) = remove_file(&mut ctx.tx, &new_path) {
                return_str += &format!("rm: Cannot remove file '{path}'\n");
            }
        }
    }

    (ctx, return_str)
}