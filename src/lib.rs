use std::net::TcpStream;
use std::result;
use std::error::Error;
pub type SdResult<T> = result::Result<T, Box<dyn Error>>;

pub mod logger;
pub mod disk;
mod models;
mod services;

// run in seperated thread
pub fn handle(stream: TcpStream) {
    // extract stream to http
    // call the corresponding service
}