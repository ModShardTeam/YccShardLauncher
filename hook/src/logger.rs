use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::OnceLock;

pub static FOLDER: OnceLock<PathBuf> = OnceLock::new();

macro_rules! log {
    ($($t:tt)*) => {
        crate::logger::log(&format!($($t)*))
    };
}

#[allow(unused_macros, reason = "used for debugging")]
macro_rules! dump {
    ($e: expr) => {
        log!("{}", crate::logger::hex_dump($e))
    };
}

pub fn log(msg: &str) {
    let Some(folder) = FOLDER.get() else {
        return;
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join("hook.log"))
    {
        let _ = writeln!(f, "{msg}");
    }
}

#[allow(dead_code, reason = "used for debugging")]
pub fn hex_dump(data: &[u8]) -> String {
    let mut str = String::new();
    for b in data {
        let _ = write!(&mut str, "{b:#x} ");
    }
    str
}
