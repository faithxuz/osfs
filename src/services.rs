pub mod permission;

mod utils;

mod info;
pub use info::info;

mod cd;
pub use cd::cd;

mod ls;
pub use ls::ls;

mod mkdir;
pub use mkdir::mkdir;

mod touch;
pub use touch::touch;

mod cat;
pub use cat::cat;

mod cp;
pub use cp::cp;

mod rm;
pub use rm::rm;

use std::sync::mpsc::Sender;
use super::fs::FsReq;

pub struct Context {
    pub uid: u8,
    pub wd: String,
    pub tx: Sender<FsReq>,
}