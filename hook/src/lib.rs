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

use std::ffi::c_void;
use std::path::PathBuf;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

#[dll_syringe::payload_procedure]
fn init(log_folder: String) -> Result<(), String> {
    let mut log_path = PathBuf::new();
    log_path.push(log_folder);
    if logger::FOLDER.set(log_path).is_err() {
        return Err("Couldnt set the log folder".to_string());
    }

    log!("--- new session ---");
    std::panic::set_hook(Box::new(|info| log!("panic: {info}")));
    let Ok(handle) = (unsafe { GetModuleHandleW(None) }) else {
        return Err("GetModuleHandleW failed".to_string());
    };

    let base = pe_structs::Va(handle.0 as usize);
    log!("base = {base:#x}");

    let vars = pe_parser::scan_var_table(base).map_err(|e| format!("{:?}", e))?;
    logger::export_symbol_table(
        "vars.json",
        vars.iter().map(|(s, &v)| (s, v.rva_from_base(base))),
    )
    .map_err(|e| format!("{:?}", e))?;

    let fns = pe_parser::scan_fn_table(base).map_err(|e| format!("{:?}", e))?;
    logger::export_symbol_table(
        "fns.json",
        fns.iter().map(|(s, f)| (s, f.start.rva_from_base(base))),
    )
    .map_err(|e| format!("{:?}", e))?;

    if symbols::VARS.set(vars).is_err() {
        return Err("Couldnt set vars map".to_string());
    }
    if symbols::FNS.set(fns).is_err() {
        return Err("Couldnt set fns map".to_string());
    }

    // Add a postfix
    // TODO: improve syntax so install / add_* is not done manually
    // let slot = (unsafe { hooks::install("gml_Object_o_player_Step_2") })
    //     .map_err(|e| format!("{:?}", e))?;
    // slot.add_postfix(Box::new(mods::my_step));

    // Add a constant patch
    mods::my_constant_patch();

    log!("hook enabled");
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_h: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        // do nothing
    }
    1
}
