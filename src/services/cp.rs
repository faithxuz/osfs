/* cp 源文件 目标文件（夹）
 * cp 源文件1 源文件2 目标文件夹 或 cp 文件* 目标文件夹
 * cp -r 源文件夹 目标文件夹
 * "-v" example: '/etc/fstab' -> '/mnt/backup/fstab'
 * "-u": for directory
 * 
 * ---(=2)
 * cp src tgt
 * if tgt exists
 *     if tgt is dir
 *         copy(src, tgt/src)
 *     else
 *         return err
 * else
 *     return copy(src, tgt)
 * 
 * ---(>2)
 * cp src1 src2 ... tgt
 * if tgt exists
 *     if tgt is dir
 *         iterate in srcs:
 *            + copy(src, tgt/src)
 *     else
 *         return err
 * else
 *     return err
 * 
 * ---fn copy(src, new_path, -r, -v) -> str
 *   if src is dir
 *       if -r is not specified
 *           return err str
 *       (-v: +) recursively copy dir to new_path
 *   else // src is file
 *       (-v: +) copy file to new_path
 */

 // todo: cp <host> ...
use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::{metadata, open_dir, open_file, create_dir, create_file};

const USAGE: &str = "Usage: cp [-r] [-v] SOURSE DEST";
const PERMISSION_SRC: (bool, bool, bool) = (true, false, false);
const PERMISSION_TGT: (bool, bool, bool) = (false, true, false);

fn copy(ctx: &mut Context, src_path: &str, tgt_path: &str, copy_dir: bool, verbose: bool) -> String {
    let mut return_str = String::new();

    let src_meta = match metadata(&mut ctx.tx, src_path) {
        Ok(m) => m,
        Err(e) => return format!("Cannot find '{}'\n", src_path),
    };

    // check source permission
    let rwx = permission::check_permission(ctx.uid, &src_meta, PERMISSION_SRC);
    if !rwx {
        return format!("Permission denied\n");
    }

    if let Ok(_) = metadata(&mut ctx.tx, tgt_path) {
        return format!("'{}' exists\n", tgt_path);
    }
    
    if src_meta.is_dir() {
        if !copy_dir {
            return format!("cp: -r not specified; omitting directory '{}'\n", tgt_path);
        }

        let mut src_dd = match open_dir(&mut ctx.tx, &src_path) {
            Ok(dd) => dd,
            Err(e) => return format!("Cannot open directory: '{}'\n", src_path),
        };

        let mut tgt_dd = match create_dir(&mut ctx.tx, &tgt_path, ctx.uid) {
            Ok(dd) => dd,
            Err(e) => return format!("Cannot create directory: '{}'\n", tgt_path),
        };

        let src_vec = match src_dd.read() {
            Ok(v) => v,
            Err(e) => return format!("Cannot read file: '{}'\n", src_path),
        };
        
        for sub_entry in src_vec {
            if sub_entry.name == ".." || sub_entry.name == "." {
                continue;
            }

            let parent_path = src_path;
            let sub_path = match utils::convert_path_to_abs(&parent_path, &sub_entry.name) {
                Ok(p) => p,
                Err(e) => return format!("Cannot convert '{}' to absolute path\n", sub_entry.name),
            };

            copy(ctx, &sub_path, &tgt_path, copy_dir, verbose);
        }
    } else {
        let mut src_fd = match open_file(&mut ctx.tx, &src_path) {
            Ok(fd) => fd,
            Err(e) => return format!("Cannot open file: '{}'\n", src_path),
        };
        let mut tgt_fd = match create_file(&mut ctx.tx, &tgt_path, ctx.uid) {
            Ok(fd) => fd,
            Err(e) => return format!("Cannot create file: '{}'\n", tgt_path),
        };

        let src_vec = match src_fd.read() {
            Ok(v) => v,
            Err(e) => return format!("Cannot read file: '{}'\n", src_path),
        };

        tgt_fd.write(&src_vec);
    }

    if verbose {
        return_str = format!("'{}' -> '{}'\n", src_path, tgt_path);
    }
    return return_str;
}

pub fn cp(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    let mut opts = Options::new();
    opts.optflag("r", "", "Copy a directory");
    opts.optflag("v", "", "Enable verbose output");

    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.free.is_empty() {
        return (ctx, String::from(USAGE));
    }

    let copy_dir = matches.opt_present("r");
    let verbose = matches.opt_present("v");

    let mut return_str = String::new();
    let (r, w, x) = (false, false, true);

    let len = matches.free.len();
    match len {
        0 => return (ctx, format!("Missing src path. {USAGE}")),
        1 => return (ctx, format!("Missing tgt path. {USAGE}")),
        2 => {
            let src_path = match utils::convert_path_to_abs(&ctx.wd, &matches.free[0]) {
                Ok(p) => p,
                Err(e) => return (ctx, format!("Cannot convert '{}' to absolute path\n", &matches.free[0])),
            };
            let tgt_path = match utils::convert_path_to_abs(&ctx.wd, &matches.free[1]) {
                Ok(p) => p,
                Err(e) => return (ctx, format!("Cannot convert '{}' to absolute path\n", &matches.free[1])),
            };

            let tgt_meta = match metadata(&mut ctx.tx, &tgt_path) {
                Ok(m) => {
                    // check target permission
                    let rwx = permission::check_permission(ctx.uid, &m, PERMISSION_TGT);
                    if !rwx {
                        return_str += &format!("Permission denied\n");
                    }

                    if m.is_dir() {
                        let src_path_append = src_path
                            .rsplit('/')
                            .next()
                            .unwrap_or(&src_path)
                            .to_string();
                        let tgt_path_new = format!("'{}'/'{}'\n", tgt_path, src_path_append);
                        let tmp = copy(&mut ctx, &src_path, &tgt_path_new, copy_dir, verbose);
                        return (ctx, tmp);
                    } else {
                        let tmp = copy(&mut ctx, &src_path, &tgt_path, copy_dir, verbose);
                        return (ctx, tmp);
                    }
                }
                Err(e) => {
                    let tmp = copy(&mut ctx, &src_path, &tgt_path, copy_dir, verbose);
                    return (ctx, tmp)
                }
            };
        }
        _ => {
            let tgt_path = match utils::convert_path_to_abs(&ctx.wd, &matches.free[len - 1]) {
                Ok(p) => p,
                Err(e) => return (ctx, format!("Cannot convert '{}' to absolute path\n\n", &matches.free[len - 1])),
            };

            let tgt_meta = match metadata(&mut ctx.tx, &tgt_path) {
                Ok(m) => {
                    // check target permission
                    let rwx = permission::check_permission(ctx.uid, &m, PERMISSION_TGT);
                    if !rwx {
                        return_str += &format!("Permission denied\n");
                    }

                    if m.is_dir() {
                        for src in &matches.free[0..len - 1] {
                            let src_path = match utils::convert_path_to_abs(&ctx.wd, &src) {
                                Ok(p) => p,
                                Err(e) => {
                                    return_str += &format!("Cannot convert '{}' to absolute path\n\n", src);
                                    continue;
                                }
                            };
                            let src_path_append = src_path
                                .rsplit('/')
                                .next()
                                .unwrap_or(&src_path)
                                .to_string();
                            let tgt_path_new = format!("'{}'/'{}'\n", tgt_path, src_path_append);
                            return_str += &copy(&mut ctx, &src_path, &tgt_path_new, copy_dir, verbose);
                        }
                        return (ctx, return_str);
                    } else {
                        return (ctx, format!("'{}' is not a directory\n\n", &matches.free[len - 1]));
                    }
                }
                Err(e) => return (ctx, format!("'{}' doesn't exist\n\n", &matches.free[len - 1])),
            };
        }
    }
}