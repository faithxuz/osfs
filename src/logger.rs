// [PASS]

use chrono::prelude::*;

/// Add a timestamp like `[hh:mm:ss] ` before the message to print.
/// 
/// ## Usage
/// 
/// ```rust
/// use crate::logger;
/// 
/// logger::log("String literal");
/// logger::log(&format!("to format: {}", 10));
/// ```
pub fn log(msg: &str) {
    let now = Local::now();
    println!("[{:0>2}:{:0>2}:{:0>2}] {msg}", now.hour(), now.minute(), now.second());
}