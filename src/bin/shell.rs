use simdisk::{PORT, SdReq, SdRes};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use serde_json;

#[derive(Debug)]
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
        self.wd = String::from(path);
    }
}

fn main() {
    connect();
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
            Err(_) => print("Not a number!\n")
        }
    }
    loop {
        print(&format!("user{}:{} $ ", ctx.user, ctx.wd));
        read(&mut buf);
        let input = parse(&buf);
        let cmd = match input.get(0) {
            Some(c) => *c,
            None => continue
        };
        if cmd.to_ascii_lowercase() == "exit" {
            break;
        }

        // send request to simdisk: ctx + args
        // and receive response
        // output the result
        print(&send(&mut ctx, cmd, &input[1..]));
    }
}

fn print(s: &str) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(s.as_bytes()).unwrap();
    stdout.flush().unwrap();
}

fn connect() -> TcpStream {
    // connect to simdisk
    let addr = SocketAddr::from(([127,0,0,1],PORT));
    match TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(30)) {
        Ok(s) => s,
        Err(_) => {
            print("Cannot connect to simdisk!");
            std::process::exit(1);
        }
    }
}

fn read(buf: &mut String) {
    buf.clear();
    let mut stdin = io::stdin().lock();
    stdin.read_line(buf).unwrap();
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

fn send(
    ctx: &mut Context,
    cmd: &str,
    args: &[&str]
) -> String {
    let mut conn = connect();
    let mut v_args = Vec::<String>::new();
    for arg in args {
        v_args.push(String::from(*arg));
    }
    let msg = SdReq {
        uid: ctx.user,
        wd: ctx.wd.clone(),
        cmd: String::from(cmd),
        args: v_args
    };
    let mut s_msg = serde_json::to_string(&msg).unwrap();
    s_msg = s_msg + "\n";

    // send request
    if let Err(e) = conn.write_all(s_msg.as_bytes()) {
        return format!("{e}\n");
    }
    if let Err(e) = conn.flush() {
        return format!("{e}\n");
    }

    // read response
    let mut res = String::new();
    let mut reader = BufReader::new(&conn);
    if let Err(e) = reader.read_line(&mut res) {
        return format!("{e}\n");
    }
    let res: SdRes = match serde_json::from_str(&res) {
        Ok(obj) => obj,
        Err(e) => return format!("{e}\n")
    };

    // workding directory may change
    if ctx.wd != res.wd {
        ctx.move_to(&res.wd);
    }

    res.result
}