use time::OffsetDateTime;

pub fn log(msg: &str) {
    let now = OffsetDateTime::now_local().unwrap().to_hms();
    println!("[{}:{}:{}] {msg}", now.0, now.1, now.2);
}