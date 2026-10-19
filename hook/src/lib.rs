#[macro_use]
mod logger;
mod patch;

mod hooks;
mod pe;
mod runtime;
mod script;

use anyhow::{Context, Result, bail};
use rhai::{CallFnOptions, Engine, Scope};
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

static ENGINE: OnceLock<Engine> = OnceLock::new();

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

    if ENGINE.set(script::engine::create_rhai_engine()).is_err() {
        bail!("Couldnt set rhai engine");
    }

    log!("--- new session ---");
    std::panic::set_hook(Box::new(|info| log!("panic: {info}")));
    let Ok(handle) = (unsafe { GetModuleHandleW(None) }) else {
        bail!("GetModuleHandleW failed");
    };

    let base = pe::addr::Va(handle.0 as usize);
    log!("base = {base:#x}");

    let vars = runtime::scan::scan_var_table(base).context("scanning var table")?;
    logger::export_symbol_table(
        "vars.json",
        vars.iter().map(|(s, &v)| (s, v.rva_from_base(base))),
    )
    .context("exporting var table")?;

    let fns = runtime::scan::scan_fn_table(base).context("scanning fn table")?;
    logger::export_symbol_table(
        "fns.json",
        fns.iter().map(|(s, f)| (s, f.start.rva_from_base(base))),
    )
    .context("exporting fn table")?;

    if runtime::symbols::VARS.set(vars).is_err() {
        bail!("couldnt set vars map");
    }
    if runtime::symbols::FNS.set(fns).is_err() {
        bail!("couldnt set fns map");
    }

    // TODO: remove this example
    let example_script = "fn load(mod) {
    mod.load_script(\"gml_Object_o_player_Step_0\")
        .replace_constant(250.0, 10.0)
        .replace_constant(30.0, 60.0)
        .save();

    mod.load_script(\"gml_Object_o_player_Step_0\")
        .add_postfix(|| set_var(\"HP\", 50))
        .add_postfix(|| set_var(\"hp\", 50))
        .add_postfix(|| set_var(\"aaaaaaaaaaaaaaaaaaaaaa\", 50))
        .add_postfix(|| set_var(\"max_xp\", \"eer\"))
        .save();
}";

    let engine = ENGINE.get().context("getting rhai engine")?;
    let loader = script::loader::Loader::new("test mod");
    let ast = engine.compile(example_script)?;

    if ast
        .iter_functions()
        .any(|f| f.name == "load" && f.params.len() == 1)
    {
        let options = CallFnOptions::new().eval_ast(false);
        let result = engine.call_fn_with_options::<()>(
            options,
            &mut Scope::new(),
            &ast,
            "load",
            (loader.clone(),),
        );
        loader.deactivate();
        if let Err(e) = result {
            log!("mod {:?}: load failed {e:#}", loader.get_name());
        } else {
            loader.apply(engine, Arc::new(ast))?;
            log!("mod {:?}: load succeeded", loader.get_name());
        }
    } else {
        log!(
            "mod {:?}: load function not found, cant apply mod",
            loader.get_name()
        );
    }

    log!("hook enabled");
    Ok(())
}

#[dll_syringe::payload_procedure]
fn init(log_folder: String) -> Result<(), String> {
    init_inner(log_folder).map_err(|e| format!("{e:#}"))
}
