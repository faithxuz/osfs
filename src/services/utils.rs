// convert path to absolute path
pub fn convert_path_to_abs(mut wd: &str, path: &str) -> Result<String, & 'static str> {
    // assume wd is an absolute path

    if path.starts_with("/") {
        // already been abs path
        return Ok(String::from(path));
    }

    // split working dir and path dir
    if wd.ends_with('/') {
        wd = &wd[..wd.len()-1];
    }
    let mut wd_vec: Vec<&str> = wd.split('/').collect();
    wd_vec.drain(0..1);
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
                None => return Err("Invalid path!")
            };
            path_vec.drain(0..1);
        } else {
            break
        }
    }

    let a = wd_vec.join("/");
    let b = path_vec.join("/");

    // regroup
    let mut str = String::from("/") + &a;

    if b.len() > 0 {
        if !str.ends_with('/') {
            str += "/";
        }
        str += &b;
    }

    Ok(str)
}