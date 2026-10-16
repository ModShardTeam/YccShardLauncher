#[macro_use]
mod logger;
mod hooks;
mod mods;
mod patch;
mod pe_parser;
mod pe_structs;
mod symbols;
mod utils;
mod yyc_structs;

use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

#[dll_syringe::payload_procedure]
fn hook_info() -> (String, Option<String>) {
    (
        env!("CARGO_PKG_VERSION").to_string(),
        option_env!("GIT_HASH").map(str::to_string),
    )
}

fn init_inner(log_folder: String) -> Result<()> {
    if logger::FOLDER.set(PathBuf::from(log_folder)).is_err() {
        bail!("Couldnt set the log folder");
    }

    log!("--- new session ---");
    std::panic::set_hook(Box::new(|info| log!("panic: {info}")));
    let Ok(handle) = (unsafe { GetModuleHandleW(None) }) else {
        bail!("GetModuleHandleW failed");
    };

    let base = pe_structs::Va(handle.0 as usize);
    log!("base = {base:#x}");

    let vars = pe_parser::scan_var_table(base).context("scanning var table")?;
    logger::export_symbol_table(
        "vars.json",
        vars.iter().map(|(s, &v)| (s, v.rva_from_base(base))),
    )
    .context("exporting var table")?;

    let fns = pe_parser::scan_fn_table(base).context("scanning fn table")?;
    logger::export_symbol_table(
        "fns.json",
        fns.iter().map(|(s, f)| (s, f.start.rva_from_base(base))),
    )
    .context("exporting fn table")?;

    if symbols::VARS.set(vars).is_err() {
        bail!("Couldnt set vars map");
    }
    if symbols::FNS.set(fns).is_err() {
        bail!("Couldnt set fns map");
    }

    // Add a postfix
    // TODO: improve syntax so install / add_* is not done manually
    // let slot = (unsafe { hooks::install("gml_Object_o_player_Step_2") })?;
    // slot.add_postfix(Box::new(mods::my_step));

    // Add a constant patch
    mods::my_constant_patch();

    log!("hook enabled");
    Ok(())
}

#[dll_syringe::payload_procedure]
fn init(log_folder: String) -> Result<(), String> {
    init_inner(log_folder).map_err(|e| format!("{e:#}"))
}
