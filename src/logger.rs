use time::OffsetDateTime;

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
    let now = OffsetDateTime::now_local().unwrap().to_hms();
    println!("[{}:{}:{}] {msg}", now.0, now.1, now.2);
}