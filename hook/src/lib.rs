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
use std::ffi::c_void;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

fn init() -> Result<()> {
    log!("--- new session ---");
    std::panic::set_hook(Box::new(|info| log!("panic: {info}")));
    let handle = unsafe { GetModuleHandleW(None) }.context("GetModuleHandleW failed")?;

    let base = pe_structs::Va(handle.0 as usize);
    log!("base = {base:#x}");

    let vars = pe_parser::scan_var_table(base).context("scanning var table")?;
    logger::export_symbol_table(
        "vars.json",
        vars.iter().map(|(s, &v)| (s, v.rva_from_base(base))),
    )?;

    let fns = pe_parser::scan_fn_table(base).context("scanning fn table")?;
    logger::export_symbol_table(
        "fns.json",
        fns.iter().map(|(s, f)| (s, f.start.rva_from_base(base))),
    )?;

    if symbols::VARS.set(vars).is_err() {
        bail!("Couldnt set vars map")
    }
    if symbols::FNS.set(fns).is_err() {
        bail!("Couldnt set fns map")
    }

    // Add a postfix
    // TODO: improve syntax so install / add_* is not done manually
    let slot = unsafe { hooks::install("gml_Object_o_player_Step_2") }?;
    slot.add_postfix(Box::new(mods::my_step));

    // Add a constant patch
    mods::my_constant_patch();

    log!("hook enabled");
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_h: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let builder = std::thread::Builder::new();
        if let Err(e) = builder.spawn(|| {
            if let Err(e) = init() {
                log!("init failed: {e:#}");
            }
        }) {
            log!("thread spawning failed: {e:#}");
        }
    }
    1
}
