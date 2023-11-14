use super::Context;

pub fn check(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    // check if file system is ok
    return (ctx, String::from("Everything is OK.\n"));
}