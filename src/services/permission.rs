use crate::fs::Metadata;

pub fn check_permission(uid: u8, meta: &Metadata, permission: (bool, bool, bool)) -> bool {
    let owner = meta.owner();
    let (r, w, x) = permission;
    if uid == owner {
        let rwx = meta.permission().0; 
        (rwx.read && r) && (rwx.write && w) && (rwx.execute && x)
    } else {
        let rwx = meta.permission().0; 
        (rwx.read && r) && (rwx.write && w) && (rwx.execute && x)
    }
}