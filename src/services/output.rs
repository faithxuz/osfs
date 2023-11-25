use super::{Context, utils, permission};
use crate::fs::{metadata, open_file, create_file};
use crate::fs::FsError;

const PERMISSION: (bool, bool, bool) = (false, true, false);

pub fn output(mut ctx: Context, s: String, redirect: &str) -> String {
    if redirect == "" {
        return s;
    }

    // output s to files in redirects
    let abs_path = match utils::convert_path_to_abs(&ctx.wd, redirect) {
        Ok(s) => s,
        Err(_) => {
            return format!("shell: Cannot write to '{redirect}': Invalid redirect.\n");
        }
    };

    // open file
    let mut fd = match open_file(&mut ctx.tx, &abs_path) {
        Ok(mut f) => {
            let m = f.metadata();
            if !permission::check_permission(ctx.uid, &m, PERMISSION) {
                return format!("shell: Permission denied: '{redirect}'\n");
            }
            f
        },
        Err(e) => {
            match e {
                FsError::NotFound => {
                    // create one
                    let (parent, _) = utils::split_path(&abs_path);
                    match metadata(&mut ctx.tx, parent) {
                        Ok(m) => {
                            if !permission::check_permission(ctx.uid, &m, PERMISSION) {
                                return format!("shell: Permission denied: {redirect}\n");
                            }
                            match create_file(&mut ctx.tx, &abs_path, ctx.uid) {
                                Ok(f) => f,
                                Err(_) => {
                                    return format!("shell: Cannot write to '{redirect}': Error when creating file.\n");
                                }
                            }
                        },
                        Err(_) => {
                            return format!("shell: Cannot write to '{redirect}': No such file or directory.\n");
                        }
                    }
                },
                FsError::NotFileButDir => {
                    return format!("shell: Cannot write to '{redirect}': Is a directory.\n");
                },
                _ => {
                    return format!("shell: Cannot write to '{redirect}': Inner Error.");
                }
            }
        }
    };

    // write file
    if let Err(_) = fd.write(&s.as_bytes().to_vec()) {
        return format!("shell: Cannot write to '{redirect}': Inner Error.\n");
    }

    return String::from("\n");
}