pub fn cp(args: Vec<&str>) -> () {
    //  check write permission
    //  check if host directory
    //      yes => cp file
    //          mkdir
    //          read source file and write
    //      no => check inode
    //          a directory => cp directory
    //              mkdir
    //              read source dir and write
    //          a file => cp file
}