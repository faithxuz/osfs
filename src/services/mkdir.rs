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
use crate::fs::{metadata, create_dir};

const USAGE: &str = "Usage: mkdir [-p] [-v] <directory1> <directory2> ...";
const PERMISSION: (bool, bool, bool) = (false, true, false);

fn split_path(path: &str) -> (&str, &str) {
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

fn create_nested_directories(ctx: &mut Context, path: &str, verbose: bool) -> String {
    let mut return_str = String::new();

    let path_split: Vec<&str> = path.split('/').collect();
    let mut current_path = String::new();

    for dir in path_split {
        current_path.push('/');
        current_path.push_str(dir);
        if !current_path.is_empty() && !current_path.ends_with('/') {
            match metadata(&mut ctx.tx, &current_path) {
                Ok(m) => {
                    let rwx = permission::check_permission(ctx.uid, &m, PERMISSION);
                    if !rwx {
                        return_str += &format!("Permission denied\n");
                        break;
                    }
                }
                Err(e) => {
                    match create_dir(&mut ctx.tx, &current_path, ctx.uid) {
                        Ok(_) => {
                            if verbose {
                                return_str += &format!("mkdir: created directory '{}'\n", current_path);
                            }
                        },
                        Err(e) => return_str += &format!("Cannot create directory: '{}'\n", current_path),
                    }
                }
            }
        }
    }
    return_str
}

pub fn mkdir(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    let mut opts = Options::new();
    opts.optflag("p", "", "Create parent directories as needed");
    opts.optflag("v", "", "Print a message for each created directory");

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.free.is_empty() {
        return (ctx, String::from(USAGE));
    }

    let recursive = matches.opt_present("p");
    let verbose = matches.opt_present("v");

    let mut return_str = String::new();

    for path in &matches.free {
        let dir_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(e) => {
                return_str += &format!("Cannot convert '{}' to absolute path\n", path);
                continue;
            },
        };

        if let Ok(_) = metadata(&mut ctx.tx, &dir_path) {
            if verbose {
                return_str += &format!("mkdir: cannot create directory '{}': File exists\n", path);
            }
        }

        let (mut parent_path, mut sub_path) = split_path(&dir_path);

        match metadata(&mut ctx.tx, &parent_path) {
            Ok(m) => {
                let rwx = permission::check_permission(ctx.uid, &m, PERMISSION);
                if !rwx {
                    return_str += &format!("Permission denied\n");
                    continue;
                }

                match create_dir(&mut ctx.tx, &dir_path, ctx.uid) {
                    Ok(_) => {
                        if verbose {
                            return_str += &format!("mkdir: created directory '{}'\n", path);
                        }
                    },
                    Err(e) => return_str += &format!("Cannot create directory: '{}'\n", path),
                }
            }
            Err(e) => {
                if !recursive {
                    return_str += &format!("mkdir: cannot create directory '{}': No such file or directory\n", parent_path);
                    continue;
                }
                let tmp = create_nested_directories(&mut ctx, &dir_path, verbose);
                return (ctx, tmp);
            }
        }
    }
    (ctx, return_str)
}