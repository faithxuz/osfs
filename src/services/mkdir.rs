 // [PASS]
 /*
 * iterate path in paths:
 *     if path exists
 *         return
 *     else if parent_path exists
 *         (-v: +) create_dir(path)
 *     else
 *         if -p is not specified
 *             return err
 *         else
 *             (-v: +) recursively create_dir(path)
 */
use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::{metadata, create_dir, FsError};

// define uasge and permission
const USAGE: &str = "Usage: mkdir [-p] [-v] <directory1> <directory2> ...\n";
const PERMISSION: (bool, bool, bool) = (false, true, false);

pub fn mkdir(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    // define params
    let mut opts = Options::new();
    opts.optflag("h", "", "Help");
    opts.optflag("p", "", "Create parent directories as needed");
    opts.optflag("v", "", "Print a message for each created directory");

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
    let recursive = matches.opt_present("p");
    let verbose = matches.opt_present("v");

    let mut return_str = String::new();

    // iterate path in paths
    for path in &matches.free {
        let dir_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(e) => {
                return_str += &format!("Cannot convert \"{}\" to absolute path\n", path);
                continue;
            },
        };

        if let Ok(_) = metadata(&mut ctx.tx, &dir_path) {
            return_str += &format!("mkdir: cannot create directory \"{}\": File exists\n", path);
        }

        // split path
        let (mut parent_path, mut sub_path) = split_path(&dir_path);

        match metadata(&mut ctx.tx, &parent_path) {
            Ok(m) => {
                // check permission
                let rwx = permission::check_permission(ctx.uid, &m, PERMISSION);
                if !rwx {
                    return_str += &format!("Permission denied\n");
                    continue;
                }

                // create dir
                match create_dir(&mut ctx.tx, &dir_path, ctx.uid) {
                    Ok(_) => {
                        // add detailed info
                        if verbose {
                            return_str += &format!("mkdir: created directory \"{}\"\n", path);
                        }
                    },
                    Err(e) => return_str += &format!("Cannot create directory \"{}\"\n", path),
                }
            }
            Err(e) => {
                // return error if "-r" is not specified
                if !recursive {
                    return_str += &format!("mkdir: cannot create directory \"{}\": No such file or directory\n", parent_path);
                    continue;
                }

                // create nested dir
                let tmp = create_nested_dir(&mut ctx, &dir_path, verbose);
                return (ctx, tmp);
            }
        }
    }

    (ctx, return_str)
}

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
        ("./", &path)
    }
}

// create dir layer by layer
fn create_nested_dir(ctx: &mut Context, path: &str, verbose: bool) -> String {
    let mut return_str = String::new();

    // split path with '/'
    let mut path_split: Vec<&str> = path.split('/').collect();
    path_split.drain(0..1);
    // current path
    let mut current_path = String::new();

    // iterate dir in splited path
    for dir in path_split {
        if dir == "" {
            return_str += &format!("Invalid path \"{path}\"\n");
            break;
        }

        // add dir to current path
        current_path.push('/');
        current_path += dir;

        // if current path is a valid path
        if !current_path.is_empty() && !current_path.ends_with('/') {
            match metadata(&mut ctx.tx, &current_path) {
                Ok(m) => {
                    // check permission
                    let rwx = permission::check_permission(ctx.uid, &m, PERMISSION);
                    if !rwx {
                        return_str += &format!("Permission denied\n");
                        break;
                    }
                }
                Err(e) => {
                    // doesn\"t exist: create dir
                    match create_dir(&mut ctx.tx, &current_path, ctx.uid) {
                        Ok(_) => {
                            // add detailed info
                            if verbose {
                                return_str += &format!("mkdir: created directory \"{}\"\n", current_path);
                            }
                        },
                        Err(e) => return_str += &format!("Cannot create directory \"{}\"\n", current_path),
                    }
                }
            }
        }
    }

    return_str
}