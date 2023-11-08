use std::error::Error;
use std::sync::Arc;
use std::net::TcpStream;

pub mod logger;
mod utils;
mod sedes;
mod bitmap;
mod models;
mod services;

pub fn init() -> Result<models::Disk, Box<dyn Error>> {
    Ok(models::init()?)
}

// run in seperated thread
pub fn handle(disk: Arc<models::Disk>, stream: TcpStream) {
    // extract stream to http
    // call the corresponding service
}