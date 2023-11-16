 /*
 * iterate path in paths:
 *     if path doesn't exist
 *         return err
 *     if filename is start with '.' and -a is not specified
 *         continue
 *     if path is a dir
 *         iterate entry in dir.read()
 *             if entry_name is start with '.' and -a is not specified
 *                 continue
 *             if entry is a dir
 *                 entry_name += '/'
 *             add entry_name to vec
 *             if -l is specified
 *                 add long list to return str
 *             add filename to return str
 *             add line feed or spaces
 *     else
 *         if -l is specified
 *             add long list to return str
 *         add filename to return str
 *         add line feed or spaces
 */
use getopts::Options;
use super::{Context, utils, permission};
use crate::fs::Rwx;
use crate::fs::{metadata, open_dir};

// define uasge and permission
const USAGE: &str = "Usage: ls [-a] [-l] [name1] [name2] ...\n";
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
    opts.optflag("h", "", "Help");
    opts.optflag("a", "", "Do not ignore entries starting with .");
    opts.optflag("l", "", "Use a long listing format");

    // parse args
    let mut matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.opt_present("h") {
        return (ctx, String::from(USAGE));
    }

    let mut return_str = String::new();

    // convert parameters to bool variables
    let all = matches.opt_present("a");
    let list_format = matches.opt_present("l");

    if matches.free.is_empty() {
        matches.free.push(String::from(&ctx.wd[..]));
    }

    // iterate path in paths
    for path in &matches.free {
        let new_path = match utils::convert_path_to_abs(&ctx.wd, &path) {
            Ok(p) => p,
            Err(e) => return (ctx, format!("Cannot convert '{}' to absolute path\n", &path)),
        };

        let meta = match metadata(&mut ctx.tx, &new_path) {
            Ok(m) => m,
            Err(e) => return (ctx, format!("Cannot find '{}'\n", &path)),
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
            // get sub entris of path
            let mut new_dd = match open_dir(&mut ctx.tx, &new_path) {
                Ok(dd) => dd,
                Err(e) => return (ctx, format!("Cannot open directory: '{}'\n", &path)),
            };
            let new_vec = match new_dd.read() {
                Ok(v) => v,
                Err(e) => return (ctx, format!("Cannot read directory: '{}'\n", &path)),
            };

            // iterate entry in sub entris
            for sub_entry in new_vec {
                // get sub path
                let mut sub_name = sub_entry.name;
                let parent_path = new_path.clone();
                let sub_path = match utils::convert_path_to_abs(&parent_path, &sub_name) {
                    Ok(p) => p,
                    Err(e) => {
                        return_str += &format!("Cannot convert '{}' to absolute path\n", &sub_name);
                        continue;
                    }
                };
                let sub_meta = match metadata(&mut ctx.tx, &sub_path) {
                    Ok(m) => m,
                    Err(e) => {
                        return_str += &format!("Connot find '{}'\n", &sub_path);
                        continue;
                    }
                };
                
                match sub_name.get(0..1) {
                    // if sub path is a hidden path
                    Some(c) => {
                        if c == "." {
                            continue;
                        }
                    },
                    None => (),
                };

                let mut permission_str = String::new();

                // if sub path is a dir
                if sub_meta.is_dir() {
                    sub_name += "/";
                    permission_str += "d";
                } else {
                    permission_str += "-";
                }

                // handle different output format
                if list_format {
                    // output of long listing format
                    const MONTH: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                    let (owner_rwx, others_rwx) = sub_meta.permission();
                    permission_str += &get_rwx(&owner_rwx)[..]; 
                    permission_str += &get_rwx(&others_rwx)[..]; 
                    let owner_str = String::from("user") + &sub_meta.owner().to_string();
                    let size_str = sub_meta.size().to_string();
                    let (mo, d, h, mi) = sub_meta.timestamp();
                    let time_str = format!("{} {:>2} {:0>2}:{:0>2}", MONTH[mo as usize], d, h, mi);
                    return_str += &format!("{:>7} {:>8} {:>10} {:>12} ", permission_str, owner_str, size_str, time_str);
                }
                return_str += &sub_name;

                if list_format {
                    return_str += "\n";
                } else {
                    return_str += "  ";
                }
            }
            return_str += "\n";
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
            let filename = new_path
                .rsplit('/')
                .next()
                .unwrap_or(&new_path)
                .to_string();
        
            match filename.get(0..0) {
                // if file is a hidden file
                Some(c) => {
                    if c == "." {
                        continue;
                    }
                },
                None => (),
            };

            // handle different output format
            if list_format {
                // output of long listing format
                const MONTH: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                let (owner_rwx, others_rwx) = meta.permission();
                let mut permission_str = String::from("-");
                permission_str += &get_rwx(&owner_rwx)[..]; 
                permission_str += &get_rwx(&others_rwx)[..]; 
                let owner_str = String::from("user") + &meta.owner().to_string();
                let size_str = meta.size().to_string();
                let (mo, d, h, mi) = meta.timestamp();
                let time_str = format!("{} {:>2} {:0>2}:{:0>2}", MONTH[mo as usize], d, h, mi);
                return_str += &format!("{:>7} {:>8} {:>10} {:>12} ", permission_str, owner_str, size_str, time_str);
            }
            return_str += &filename;

            return_str += "\n";
        }
    }

    (ctx, return_str)
}