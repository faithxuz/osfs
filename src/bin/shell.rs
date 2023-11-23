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
    let mut ctx = Context::new(0);
 
    // login
    let mut try_count = 0;
    loop {
        if try_count >= 3 {
            std::process::exit(1);
        }
        try_count += 1;
        print("login (id in 0~255): ");
        let buf = read_raw();
        match buf.parse::<i64>() {
            Ok(id) => if id >= 0 && id < 256 {
                ctx = Context::new(id as u8);
                let login = send(&mut ctx, String::from("login"), Vec::new(), Vec::new());
                if login != "" {
                    print(&login);
                    std::process::exit(1);
                }
                break;
            }
            else {
                print("Your id is less than 0 or greater than 255!\n");
            }
            Err(_) => print("Not a number!\n")
        }
    }

    // main loop
    loop {
        print(&format!("user{}:{} $ ", ctx.user, ctx.wd));
        let mut input = read();
        if input.0.len() == 0 {
            continue;
        }
        let mut cmd = String::new();
        for s in input.0.drain(0..1) {
            cmd = s;
            break;
        }
        if cmd.to_ascii_lowercase() == "exit" {
            break;
        }

        // send request to simdisk: ctx + args + redirects
        // and receive response
        // output the result
        print(&send(&mut ctx, cmd, input.0, input.1));
    }
}

fn print(s: &str) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(s.as_bytes()).unwrap();
    stdout.flush().unwrap();
}

fn read() -> (Vec<String>, Vec<String>) {
    let mut args = Vec::<String>::new();
    let mut redirects = Vec::<String>::new();
    let line = read_raw();

    // parse words
    let mut word = String::new();
    let mut it = line.chars();
    let mut quote_flag = false;
    let mut red_flag = false;
    while let Some(c) = it.next() {
        if c.is_whitespace() {
            if quote_flag {
                word.push(c);
                continue;
            } else if word != "" {
                if red_flag {
                    redirects.push(word);
                } else {
                    args.push(word);
                }
                word = String::new();
            }
        } else {
            match c {
                '>' => {
                    if quote_flag {
                        word.push(c);
                    } else {
                        if word != "" {
                            args.push(word);
                            word = String::new();
                        }
                        red_flag = true;
                    }
                }
                '"' => {
                    if word != "" {
                        if red_flag {
                            redirects.push(word);
                        } else {
                            args.push(word);
                        }
                        word = String::new();
                    }
                    quote_flag = !quote_flag;
                },
                _ => word.push(c)
            }
        }
    }
    if word != "" {
        if red_flag {
            redirects.push(word);
        } else {
            args.push(word);
        }
    }
    (args, redirects)
}

fn read_raw() -> String {
    let mut buf = String::new();
    let mut stdin = io::stdin().lock();
    stdin.read_line(&mut buf).unwrap();
    if buf.ends_with('\n') {
        buf.pop();
        if buf.ends_with('\r') {
            buf.pop();
        }
    }
    buf
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

fn send(
    ctx: &mut Context,
    cmd: String,
    args: Vec<String>,
    redirects: Vec<String>
) -> String {
    let mut conn = connect();
    let msg = SdReq {
        uid: ctx.user,
        wd: ctx.wd.clone(),
        cmd, args, redirects
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