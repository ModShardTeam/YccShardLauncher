use std::{
    ffi::c_void,
    mem,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::Result;
use rhai::{AST, Engine, EvalAltResult, FnPtr};

use crate::{hooks, patch, symbols, utils};

#[derive(Clone)]
pub struct Loader(Arc<LoaderState>);

enum Op {
    Replace(f64, f64),
    Postfix(FnPtr),
    Prefix(FnPtr),
}

struct ScriptChanges {
    target_script_name: String,
    ops: Vec<Op>,
}

struct LoaderState {
    mod_name: String,
    active: AtomicBool,
    pending: Mutex<Vec<ScriptChanges>>,
}
/*

impl Context {
    fn set_var(&self, var_name: &str, value: f64) -> Result<(), Box<EvalAltResult>> {
        let Some(vars) = symbols::VARS.get() else {
            return Err("VARS table not initialized yet".into());
        };
        if let Err(e) = unsafe {
            utils::set_var(
                vars,
                self.0.this,
                var_name,
                &utils::Variant::Real(value),
            )
        } {
            // TODO: ignore IdNotReady, but it's a non-issue currently, IdNotReady is not seen in the log
            log!("{:#?}", e);
        }
        Ok(())
    }
} */

impl Loader {
    pub fn new(mod_name: &str) -> Self {
        Loader(Arc::new(LoaderState {
            mod_name: mod_name.to_string(),
            active: AtomicBool::new(true),
            pending: Mutex::new(vec![]),
        }))
    }

    pub fn get_name(&self) -> &str {
        &self.0.mod_name
    }

    pub fn deactivate(&self) {
        self.0.active.store(false, Ordering::Relaxed);
    }

    fn check_active(&self) -> Result<(), Box<EvalAltResult>> {
        if self.0.active.load(Ordering::Relaxed) {
            Ok(())
        } else {
            Err(format!("mod {}: loader used outside of load()", self.get_name()).into())
        }
    }

    fn load_script(&mut self, script_name: &str) -> Result<ScriptLoader, Box<EvalAltResult>> {
        self.check_active()?;
        Ok(ScriptLoader {
            loader: self.clone(),
            script_name: script_name.to_owned(),
            ops: Arc::new(Mutex::new(vec![])),
        })
    }

    pub fn apply(&self, engine: &'static Engine, ast: Arc<AST>) -> anyhow::Result<()> {
        let changes = mem::take(
            &mut *self
                .0
                .pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );

        for change in changes {
            // TODO: remove this for patch-only function as it burned a slot for nothing
            let slot = unsafe { hooks::install(&change.target_script_name) }?;
            for op in change.ops {
                match op {
                    Op::Replace(old, new) => {
                        let mod_name = change.target_script_name.to_owned();
                        patch::patch_constant(&mod_name, old, new)?;
                    }
                    Op::Postfix(postfix) => {
                        let mod_name = change.target_script_name.to_owned();
                        let ast = Arc::clone(&ast);
                        slot.add_postfix(Box::new(move |_this, _other| {
                            if let Err(e) = postfix.call::<()>(engine, &ast, ()) {
                                log!("mod {}: postfix failed: {e:#}", mod_name);
                            }
                        }));
                    }
                    Op::Prefix(prefix) => {
                        let mod_name = change.target_script_name.to_owned();
                        let ast = Arc::clone(&ast);
                        slot.add_prefix(Box::new(move |_this, _other| {
                            match prefix.call::<bool>(engine, &ast, ()) {
                                Err(e) => {
                                    log!("mod {}: postfix failed: {e:#}", mod_name);
                                    None
                                }
                                Ok(v) => Some(v),
                            }
                        }));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct ScriptLoader {
    loader: Loader,
    script_name: String,
    ops: Arc<Mutex<Vec<Op>>>,
}

impl ScriptLoader {
    fn replace_constant(&mut self, old: f64, new: f64) -> Result<Self, Box<EvalAltResult>> {
        self.loader.check_active()?;
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Op::Replace(old, new));
        Ok(self.clone())
    }

    fn add_postfix(&mut self, callback: FnPtr) -> Result<Self, Box<EvalAltResult>> {
        self.loader.check_active()?;
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Op::Postfix(callback));
        Ok(self.clone())
    }

    fn add_prefix(&mut self, callback: FnPtr) -> Result<Self, Box<EvalAltResult>> {
        self.loader.check_active()?;
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Op::Prefix(callback));
        Ok(self.clone())
    }

    fn save(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.loader.check_active()?;

        if !symbols::is_function_name_valid(&self.script_name) {
            return Err(format!("{} does not exist", self.script_name).into());
        }

        let mut ops = self.ops.lock().unwrap_or_else(PoisonError::into_inner);
        for op in &*ops {
            match op {
                Op::Replace(old, _) => {
                    if let Err(e) = symbols::find_imm64(&self.script_name, *old) {
                        return Err(format!("{e:#} in {}", self.script_name).into());
                    }
                }
                Op::Postfix(_fn_ptr) => {}
                Op::Prefix(_fn_ptr) => {}
            }
        }

        if !ops.is_empty() {
            self.loader
                .0
                .pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(ScriptChanges {
                    target_script_name: self.script_name.clone(),
                    ops: mem::take(&mut ops),
                });
        }

        Ok(())
    }
}

pub fn create_rhai_engine() -> Engine {
    // TODO: redirects print and debug to the log
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_string_size(10_000);
    engine.set_max_array_size(1_000);
    engine.set_max_map_size(1_000);
    engine.disable_symbol("eval");

    engine
        .register_type_with_name::<Loader>("Loader")
        .register_type_with_name::<ScriptLoader>("ScriptLoader")
        .register_fn("load_script", Loader::load_script)
        .register_fn("replace_constant", ScriptLoader::replace_constant)
        .register_fn("add_postfix", ScriptLoader::add_postfix)
        .register_fn("add_prefix", ScriptLoader::add_prefix)
        .register_fn("save", ScriptLoader::save);
    engine
}
