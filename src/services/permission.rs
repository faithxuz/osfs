use crate::fs::Metadata;

// check permission
pub fn check_permission(uid: u8, meta: &Metadata, permission: (bool, bool, bool)) -> bool {
    // get owner
    let owner = meta.owner();
    let (r, w, x) = permission;

    // if user is the owner
    if uid == owner {
        let rwx = meta.permission().0; 
        (rwx.read && r) && (rwx.write && w) && (rwx.execute && x)
    } else {
        // if user is an other
        let rwx = meta.permission().1; 
        (rwx.read && r) && (rwx.write && w) && (rwx.execute && x)
    }
}