use super::Context;

pub fn info(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    // get file system info
    let return_str = String::from("
        Disk Struture
        size: 128MB
        block size: 1KB
        block count: 128 * 1024
        superblock: 1 blocks
        inode bitmap: 1 blocks
        inode size: 64B
        inode count: 4096
        inode: 256 blocks
        data bitmap: 16 blocks
    ");

    return (ctx, return_str);
}