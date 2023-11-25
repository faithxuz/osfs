use getopts::Options;
use super::Context;

// define uasge and permission
const USAGE: &str = "Usage: echo [-ne] [arg ...]\n";

fn interpret_escape_characters(input: String) -> String {
    // deal with escape characters
    input.replace("\\n", "\n")
         .replace("\\t", "\t")
         .replace("\\r", "\r")
         .replace("\\\\", "\\")
         .replace("\\\"", "\"")
}

pub fn echo(mut ctx: Context, args: Vec<&str>) -> (Context, String) {
    if args.len() < 1 {
        return (ctx, String::from(USAGE));
    }

    // define params
    let mut opts = Options::new();
    opts.optflag("h", "", "Help");
    opts.optflag("n", "", "Do not append a newline");
    opts.optflag("e", "", "Enable interpretation of the following backslash escapes");

    // parse args
    let matches = match opts.parse(&args) {
        Ok(m) => m,
        Err(f) => {
            return (ctx, f.to_string());
        }
    };

    if matches.opt_present("h") {
        return (ctx, String::from(USAGE));
    }

    // convert parameters to bool variables
    let no_newline = matches.opt_present("n");
    let interpret_esc = matches.opt_present("e");

    let mut return_str = String::new();

    for arg in &matches.free {
        return_str.push_str(arg);
        return_str.push(' ');
    }
    return_str.push('\n');
    
    // delete ' ' at the end
    if !return_str.is_empty() {
        return_str.pop();
    }

    if !no_newline {
        return_str.push('\n');
    }

    if interpret_esc {
        return_str = interpret_escape_characters(return_str);
    }

    (ctx, return_str)
}