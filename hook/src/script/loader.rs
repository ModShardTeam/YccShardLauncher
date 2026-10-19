use std::{
    mem,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
};

use rhai::{AST, Engine, EvalAltResult, FnPtr};

use crate::{hooks, patch, runtime::symbols};

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

#[derive(Clone)]
pub struct Loader(Arc<LoaderState>);

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
            Err(format!("mod {:?}: loader used outside of load()", self.get_name()).into())
        }
    }

    pub fn load_script(&mut self, script_name: &str) -> Result<ScriptLoader, Box<EvalAltResult>> {
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
                let mod_name = self.get_name().to_owned();
                let func_name = change.target_script_name.to_owned();
                match op {
                    Op::Replace(old, new) => {
                        patch::patch_constant(&func_name, old, new)?;
                    }
                    Op::Postfix(postfix) => {
                        let ast = Arc::clone(&ast);
                        slot.add_postfix(Box::new(move || {
                            if let Err(e) = postfix.call::<()>(engine, &ast, ()) {
                                log!(
                                    "mod {:?}: postfix on {:?} failed: {e:#}",
                                    mod_name,
                                    func_name
                                );
                            }
                        }));
                    }
                    Op::Prefix(prefix) => {
                        let ast = Arc::clone(&ast);
                        slot.add_prefix(Box::new(move || {
                            match prefix.call::<bool>(engine, &ast, ()) {
                                Err(e) => {
                                    log!(
                                        "mod {:?}: prefix on {:?} failed: {e:#}",
                                        mod_name,
                                        func_name
                                    );
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
pub struct ScriptLoader {
    loader: Loader,
    script_name: String,
    ops: Arc<Mutex<Vec<Op>>>,
}

impl ScriptLoader {
    pub fn replace_constant(&mut self, old: f64, new: f64) -> Result<Self, Box<EvalAltResult>> {
        self.loader.check_active()?;
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Op::Replace(old, new));
        Ok(self.clone())
    }

    pub fn add_postfix(&mut self, callback: FnPtr) -> Result<Self, Box<EvalAltResult>> {
        self.loader.check_active()?;
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Op::Postfix(callback));
        Ok(self.clone())
    }

    pub fn add_prefix(&mut self, callback: FnPtr) -> Result<Self, Box<EvalAltResult>> {
        self.loader.check_active()?;
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Op::Prefix(callback));
        Ok(self.clone())
    }

    pub fn save(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.loader.check_active()?;

        if !symbols::is_function_name_valid(&self.script_name) {
            return Err(format!("{} does not exist", self.script_name).into());
        }

        let mut ops = self.ops.lock().unwrap_or_else(PoisonError::into_inner);
        for op in &*ops {
            match op {
                Op::Replace(old, _) => {
                    if let Err(e) = patch::find_imm64(&self.script_name, *old) {
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
