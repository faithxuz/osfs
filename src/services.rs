mod permission;
mod utils;
mod output;
mod info;
mod cd;
mod ls;
mod mkdir;
mod touch;
mod cat;
mod cp;
mod rm;
mod check;

pub use {
    output::output,
    info::info,
    cd::cd,
    ls::ls,
    mkdir::mkdir,
    touch::touch,
    cat::cat,
    cp::cp,
    rm::rm,
    check::check,
};

pub struct Context {
    pub uid: u8,
    pub wd: String,
    pub tx: std::sync::mpsc::Sender<super::fs::FsReq>,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            uid: self.uid,
            wd: self.wd.clone(),
            tx: self.tx.clone(),
        }
    }
}