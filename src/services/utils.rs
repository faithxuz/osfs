// convert path to absolute path
pub fn convert_path_to_abs(wd: &str, path: &str) -> Result<String, & 'static str> {
    if path.starts_with("/") {
        // already been abs path
        return Ok(String::from(path));
    }

    // split working dir and path dir
    let mut wd_vec: Vec<&str> = wd.split('/').collect();
    let mut path_vec: Vec<&str> = path.split('/').collect();

    loop {
        // get the first char
        let tmp = match path_vec.get(0) {
            Some(s) => *s,
            None => break
        };
        // handle "." or ".."
        if tmp == "." {
            path_vec.drain(0..1);
            continue
        } else if tmp == ".." {
            match wd_vec.pop() {
                Some(_) => (),
                None => todo!() // ERR
            };
            path_vec.drain(0..1);
        } else {
            break
        }
    }

    let a = wd_vec.join("/");
    let mut b = path_vec.join("/");

    // delete the last '/'
    if b.ends_with('/') {
        b.pop();
    }

    // regroup
    return Ok(String::from(a + "/" + &b));
}