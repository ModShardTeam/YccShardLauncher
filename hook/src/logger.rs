use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use crate::pe_structs::Rva;

// TODO: should be passed by the injector
const FOLDER: &str = r"D:\yyc_exploration\temp";

macro_rules! log {
    ($($t:tt)*) => {
        crate::logger::log(&format!($($t)*))
    };
}

pub fn log(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(FOLDER).join("hook.log"))
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
    table: impl Iterator<Item = (&'a String, Option<Rva>)>,
) -> Result<()> {
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

    std::fs::write(Path::new(FOLDER).join(file), msg)
        .with_context(|| format!("Writing table in {} failed", file))
}
