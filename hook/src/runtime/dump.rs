use std::collections::BTreeMap;

use anyhow::{Context, Result};

use crate::{logger::FOLDER, pe::addr};

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

    std::fs::write(folder.join(file), msg)
        .with_context(|| format!("Writing table in {} failed", file))
}
