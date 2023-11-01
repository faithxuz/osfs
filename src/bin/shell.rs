use std::io::{self, BufRead, Write};

struct Context {
    user: u8,
    wd: String
}

impl Context {
    pub fn new(u: u8) -> Self {
        Self {
            user: u,
            wd: String::from("/")
        }
    }

    pub fn move_to(&mut self, path: &str) {
        self.wd.clear();
        self.wd.insert_str(0, path);
    }
}

fn print(s: &str) {
    let stdout = io::stdout();
    {
        let mut lock = stdout.lock();
        lock.write_all(s.as_bytes()).unwrap();
        lock.flush().unwrap();
    }
}

fn read(buf: &mut String) {
    buf.clear();
    let stdin = io::stdin();
    {
        let mut lock = stdin.lock();
        lock.read_line(buf).unwrap();
    }
    if buf.ends_with('\n') {
        buf.pop();
        if buf.ends_with('\r') {
            buf.pop();
        }
    }
}

fn parse(input: &str) -> Vec<&str> {
    input.split_ascii_whitespace().collect()
}

fn init() {
    // connect to simdisk
}

fn main() {
    init();
    let mut ctx: Context;
    let mut buf = String::new();
    loop {
        print("login (id in 0~255): ");
        read(&mut buf);
        match buf.parse::<i64>() {
            Ok(id) => if id >= 0 && id < 256 {
                ctx = Context::new(id as u8);
                break;
            }
            else {
                print("Your id is less than 0 or greater than 255!\n");
            }
            Err(e) => print("Not a number!\n")
        }
    }
    loop {
        print(&format!("user{}:{} $ ", ctx.user, ctx.wd));
        read(&mut buf);
        let args = parse(&buf);
        if args[0] == "exit" {
            break;
        }
        print(&format!("you inputted: {args:?}\n"));
        // send request to simdisk: ctx + args
        // and receive response
        // output the result
        // workding directory may change
    }
}