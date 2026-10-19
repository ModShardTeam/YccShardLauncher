use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::pe::addr;

pub static FOLDER: OnceLock<PathBuf> = OnceLock::new();

macro_rules! log {
    ($($t:tt)*) => {
        crate::logger::log(&format!($($t)*))
    };
}

pub fn log(msg: &str) {
    let Some(folder) = FOLDER.get() else {
        return;
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(folder).join("hook.log"))
    {
        let _ = writeln!(f, "{msg}");
    }
}

pub fn hex_dump(data: &[u8]) -> String {
    let mut str = String::new();
    for b in data {
        let _ = write!(&mut str, "{b:#x} ");
    }
    str
}

pub fn export_symbol_table<'a>(
    file: &str,
    table: impl Iterator<Item = (&'a String, Option<addr::Rva>)>,
) -> Result<()> {
    let folder = FOLDER.get().with_context(|| "Log folder is not set")?;
    let msg = serde_json::to_string_pretty(
        &table
            .map(|(k, v)| {
                (
                    k,
                    match v {
                        None => "#NA#".to_string(),
                        Some(rva) => format!("{}", rva),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    )?;

    std::fs::write(Path::new(folder).join(file), msg)
        .with_context(|| format!("Writing table in {} failed", file))
}
