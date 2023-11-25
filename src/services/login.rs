use super::{Context, permission};
use crate::fs::{open_dir, create_dir, remove_dir, remove_file, metadata};
use crate::fs::FsError;

const PERMISSION: (bool, bool, bool) = (true, true, true);

pub fn login(mut ctx: Context, _: Vec<&str>) -> (Context, String) {
    // found or create home_path
    let home_path = "/home";
    match open_dir(&mut ctx.tx, home_path) {
        Ok(_) => (),
        Err(e) => {
            match e {
                FsError::NotFound => {
                    if let Err(_) = create_dir(&mut ctx.tx, home_path, 0) {
                        return (ctx, String::from("Cannot login! Failed to found home!\n"))
                    }
                },
                FsError::NotDirButFile => {
                    if let Err(_) = remove_file(&mut ctx.tx, home_path) {
                        return (ctx, String::from("Cannot login! Failed to found home!\n"))
                    }
                    if let Err(_) = create_dir(&mut ctx.tx, home_path, 0) {
                        return (ctx, String::from("Cannot login! Failed to found home!\n"))
                    }
                },
                _ => return (ctx, String::from("Cannot login! Failed to found home!\n"))
            }
        }
    };

    // found or create "/home/<uid>"
    let path = format!("{home_path}/{}", ctx.uid.to_string());
    match metadata(&mut ctx.tx, &path) {
        Ok(m) => {
            // check permission
            if !permission::check_permission(ctx.uid, &m, PERMISSION) {
                if let Err(_) = remove_dir(&mut ctx.tx, &path) {
                    return (ctx, String::from("Cannot login! Failed to found home!\n"))
                }
                if let Err(_) = create_dir(&mut ctx.tx, &path, ctx.uid) {
                    return (ctx, String::from("Cannot login! Failed to found home!\n"))
                }
            }
        },
        Err(e) => {
            match e {
                FsError::NotFound => {
                    if let Err(_) = create_dir(&mut ctx.tx, &path, ctx.uid) {
                        return (ctx, String::from("Cannot login! Failed to found home!\n"))
                    }
                },
                FsError::NotDirButFile => {
                    if let Err(_) = remove_dir(&mut ctx.tx, &path) {
                        return (ctx, String::from("Cannot login! Failed to found home!\n"))
                    }
                    if let Err(_) = create_dir(&mut ctx.tx, &path, ctx.uid) {
                        return (ctx, String::from("Cannot login! Failed to found home!\n"))
                    }
                },
                _ => return (ctx, String::from("Cannot login! Failed to found home!\n"))
            }
        }
    }

    ctx.wd = path;
    return (ctx, String::new())
}