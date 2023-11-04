use std::net::TcpStream;
use std::result;
use std::error::Error;
pub type SdResult<T> = result::Result<T, Box<dyn Error>>;

pub mod logger;
pub mod disk;
mod utils;
mod sedes;
mod bitmap;
mod models;
mod services;

pub fn init() -> SdResult<()> {
    disk::check_disk()?;
    models::init()?;
    services::init()?;
    Ok(())
}

// run in seperated thread
pub fn handle(stream: TcpStream) {
    // extract stream to http
    // call the corresponding service
}