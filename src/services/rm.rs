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
 * ---fn remove_dir_recursively(dir_path) ->
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

const USAGE: &str = "Usage: rm [-r] <name1> <name2> ...";
const PERMISSION: (bool, bool, bool) = (false, true, false);

fn remove_dir_recursively(ctx: &mut Context, dir_path: &str) -> String{
    let return_str = String::new();

    let mut dir_dd = match open_dir(&mut ctx.tx, &dir_path) {
        Ok(m) => m,
        Err(e) => return format!("Cannot find directory '{}'\n", dir_path),
    };

    let vec = match dir_dd.read() {
        Ok(v) => v,
        Err(e) => return format!("Cannot read directory '{}'\n", dir_path),
    };

    for sub_entry in vec {
        if sub_entry.name == ".." || sub_entry.name == "." {
            continue;
        }
        
        let parent_path = dir_path;
        let sub_path = match utils::convert_path_to_abs(&parent_path, &sub_entry.name) {
            Ok(p) => p,
            Err(e) => return format!("Cannot convert '{}' to absolute path\n", sub_entry.name),
        };
        
        let sub_meta = match metadata(&mut ctx.tx, &sub_path) {
            Ok(m) => m,
            Err(e) => return format!("Cannot find '{}'\n", sub_entry.name),
        };

        // check permission
        let rwx = permission::check_permission(ctx.uid, &sub_meta, PERMISSION);
        if !rwx {
            return format!("Permission denied\n");
        }

        if sub_meta.is_dir() {
            remove_dir_recursively(ctx, &sub_path);
        } else {
            if let Err(_) = remove_file(&mut ctx.tx, &sub_path) {
                return format!("Cannot remove file '{}'\n", sub_entry.name);
            }
        }
    }

    if let Err(_) = remove_dir(&mut ctx.tx, &dir_path) {
        return format!("Cannot remove directory '{}'\n", dir_path);
    }

    return_str
}

pub fn rm(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    let mut opts = Options::new();
    opts.optflag("r", "", "Remove directories and their contents recursively");

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.free.is_empty() {
        return (ctx, String::from(USAGE));
    }

    let remove_dir = matches.opt_present("r");

    let mut return_str = String::new();

    for path in &matches.free {
        let new_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(_) => {
                return_str += &format!("Cannot convert '{}' to absolute path\n", path);
                continue;
            }
        };

        let meta = match metadata(&mut ctx.tx, &new_path) {
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
        }

        if meta.is_dir() {
            if !remove_dir {
                return_str += &remove_dir_recursively(&mut ctx, &new_path);
            }
        } else {
            if let Err(_) = remove_file(&mut ctx.tx, &new_path) {
                return_str += &format!("Cannot remove file '{}'\n", path);
            }
        }
    }
    (ctx, return_str)
}