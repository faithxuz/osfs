use super::{Context, utils, permission};
use crate::fs::{metadata, open_file, create_file};
use crate::fs::FsError;

const PERMISSION: (bool, bool, bool) = (false, true, false);

pub fn output(mut ctx: Context, s: String, redirects: &Vec<String>) -> String {
    if redirects.len() == 0 {
        return s;
    }

    let mut rtn_str = String::new();
    // output s to files in redirects
    for path in redirects {
        let abs_path = match utils::convert_path_to_abs(&ctx.wd, path) {
            Ok(s) => s,
            Err(e) => {
                rtn_str += &format!("shell: Cannot write to '{path}': Invalid path.\n");
                continue;
            }
        };

        // open file
        let mut fd = match open_file(&mut ctx.tx, &abs_path) {
            Ok(mut f) => {
                let m = f.metadata();
                if !permission::check_permission(ctx.uid, &m, PERMISSION) {
                    rtn_str += &format!("shell: Permission denied: '{path}'\n");
                    continue;
                }
                f
            },
            Err(e) => {
                match e {
                    FsError::NotFound => {
                        // create one
                        let (parent, filename) = utils::split_path(path);
                        match metadata(&mut ctx.tx, parent) {
                            Ok(m) => {
                                if !permission::check_permission(ctx.uid, &m, PERMISSION) {
                                    rtn_str += &format!("shell: Permission denied: {path}\n");
                                    continue;
                                }
                                match create_file(&mut ctx.tx, path, ctx.uid) {
                                    Ok(f) => f,
                                    Err(_) => {
                                        rtn_str += &format!("shell: Cannot write to '{path}': Error when creating file.\n");
                                        continue;
                                    }
                                }
                            },
                            Err(_) => {
                                rtn_str += &format!("shell: Cannot write to '{path}': No such file or directory.\n");
                                continue;
                            }
                        }
                    },
                    FsError::NotFileButDir => {
                        rtn_str += &format!("shell: Cannot write to '{path}': Is a directory.\n");
                        continue;
                    },
                    _ => {
                        rtn_str += &format!("shell: Cannot write to '{path}': Inner Error.");
                        continue;
                    }
                }
            }
        };

        if let Err(e) = fd.write(&s.as_bytes().to_vec()) {
            rtn_str += &format!("shell: Cannot write to '{path}': Inner Error.\n");
        }
    }

    return rtn_str;
}