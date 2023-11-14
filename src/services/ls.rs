 /*
 * iterate path in paths:
 *     if path doesn't exist
 *         return err
 *     if path is a dir
 *         iterate entry in dir.read()
 *             if path_append is start with '.' and -a is not specified
 *                 continue
 *             if entry is a dir
 *                 path_append += '/'
 *             add path_append to vec
 *             if -l is specified
 *                 add long list to return str
 *             else
 *     else
 *         if path_append is start with '.' and -a is not specified
 *             continue
 *         add path_append to vec
 *         if -l is specified
 *             add long list to return str
 *         else
 */
use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::Rwx;
use crate::fs::{metadata, open_dir};

// define uasge and permission
const USAGE: &str = "Usage: ls [-a] [-l] [name1] [name2] ...";
const PERMISSION: (bool, bool, bool) = (true, false, false);

// get user's rwx and convert to string
fn get_rwx(rwx: &Rwx) -> String {
    let mut return_str = String::new();
    return_str.push(match rwx.read {
        true => 'r',
        false => '-',
    });
    return_str.push(match rwx.write {
        true => 'w',
        false => '-',
    });
    return_str.push(match rwx.execute {
        true => 'x',
        false => '-',
    });
    return_str
}

pub fn ls(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    // define params
    let mut opts = Options::new();
    opts.optflag("a", "", "Do not ignore entries starting with .");
    opts.optflag("l", "", "Use a long listing format");

    // parse args
    let mut matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    let mut return_str = String::new();
    let mut permission_str = String::new();
    let mut owner_str = String::new();
    let mut size_str = String::new();
    let mut time_str = String::new();

    // convert parameters to bool variables
    let all = matches.opt_present("a");
    let list_format = matches.opt_present("l");

    if matches.free.is_empty() {
        matches.free.push(String::from(&ctx.wd[..]));
    }

    // iterate path in paths
    for mut path in &matches.free {
        let new_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(e) => return (ctx, format!("Cannot convert '{}' to absolute path\n", path)),
        };

        let meta = match metadata(&mut ctx.tx, &new_path) {
            Ok(m) => m,
            Err(e) => return (ctx, format!("Cannot find '{}'\n", path)),
        };

        // check permission
        let rwx = permission::check_permission(ctx.uid, &meta, PERMISSION);
        if !rwx {
            return_str += &format!("Permission denied\n");
            continue;
        }

        return_str += &format!("{}:\n", path);

        // if path is a dir
        if meta.is_dir() {
            // get sub entrys of path
            let mut new_dd = match open_dir(&mut ctx.tx, &new_path) {
                Ok(dd) => dd,
                Err(e) => return (ctx, format!("Cannot open directory: '{}'\n", path)),
            };
            let new_vec = match new_dd.read() {
                Ok(v) => v,
                Err(e) => return (ctx, format!("Cannot read directory: '{}'\n", path)),
            };

            // iterate entry in sub entrys
            for sub_entry in new_vec {
                // get sub path
                let parent_path = new_path.clone();
                let sub_path = match utils::convert_path_to_abs(&parent_path, &sub_entry.name) {
                    Ok(p) => p,
                    Err(e) => {
                        return_str += &format!("Cannot convert '{}' to absolute path\n", sub_entry.name);
                        continue;
                    }
                };
                let sub_meta = match metadata(&mut ctx.tx, &sub_path) {
                    Ok(m) => m,
                    Err(e) => {
                        return_str += &format!("Connot find '{}'\n", sub_path);
                        continue;
                    }
                };
                
                // get sub path name
                let mut sub_path_append = sub_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&sub_path)
                    .to_string();
        
                match sub_path_append.chars().nth(0) {
                    // if sub path is a hidden path
                    Some(c) => {
                        if c == '.' && !all {
                            continue;
                        }
                    },
                    None => (),
                };

                // if sub path is a dir
                if sub_meta.is_dir() {
                    sub_path_append.push('/');
                }

                // output of long listing format
                permission_str.push('d');
                let (owner_rwx, others_rwx) = sub_meta.permission();
                permission_str += &get_rwx(&owner_rwx)[..]; 
                permission_str += &get_rwx(&others_rwx)[..]; 
                owner_str = String::from_utf8(vec!(sub_meta.owner())).unwrap();
                size_str = sub_meta.size().to_string();
                time_str = format!("({}, {}, {}, {})", sub_meta.timestamp().0, sub_meta.timestamp().1, 
                                                sub_meta.timestamp().2, sub_meta.timestamp().3);

                // handle different output format
                if list_format {
                    return_str += &format!("{:>7} {:>10} {:>10} {:>10}\n", permission_str, owner_str, size_str, time_str);
                } else {
                    return_str += &format!("{} ", sub_path_append);
                }
            }
            return_str += &String::from("\n");
        }
        else {
            // get file
            let new_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
                Ok(p) => p,
                Err(e) => return (ctx, format!("Cannot convert '{}' to absolute path\n", path)),
            };
            let meta = match metadata(&mut ctx.tx, &new_path) {
                Ok(m) => m,
                Err(e) => return (ctx, format!("Cannot find '{}'\n", path)),
            };

            // get file path name
            let new_path_append = new_path
                .rsplit('/')
                .next()
                .unwrap_or(&new_path)
                .to_string();
        
            match new_path_append.chars().nth(0) {
                // if file is a hidden file
                Some(c) => {
                    if c == '.' && !all {
                        continue;
                    }
                },
                None => (),
            };
            
            // output of long listing format
            permission_str.push('-');
            let (owner_rwx, others_rwx) = meta.permission();
            permission_str += &get_rwx(&owner_rwx)[..]; 
            permission_str += &get_rwx(&others_rwx)[..]; 
            owner_str = String::from_utf8(vec!(meta.owner())).unwrap();
            size_str = meta.size().to_string();
            time_str = format!("({}, {}, {}, {})", meta.timestamp().0, meta.timestamp().1, 
                                                meta.timestamp().2, meta.timestamp().3);

            // handle different output format
            if list_format {
                return_str += &format!("{:>7} {:>10} {:>10} {:>10}\n", permission_str, owner_str, size_str, time_str);
            } else {
                return_str += &format!("{} ", new_path_append);
            }
        }
        
        // add "\n"
        if !list_format {
            return_str += &String::from("\n");
        }
    }
    (ctx, return_str)
}