use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use rhai::{Engine, EvalAltResult};

use crate::patch;

#[derive(Clone, Debug)]
pub struct Loader(Arc<LoaderState>);

#[derive(Debug)]
struct LoaderState {
    mod_name: String,
    active: AtomicBool,
}

impl Loader {
    pub fn new(mod_name: &str) -> Self {
        Loader(Arc::new(LoaderState {
            mod_name: mod_name.to_string(),
            active: AtomicBool::new(true),
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

    fn patch_constant(&mut self, func: &str, old: f64, new: f64) -> Result<(), Box<EvalAltResult>> {
        self.check_active()?;

        patch::patch_constant(func, old, new)
            .map_err(|e| format!("mod {}: {e:#}", self.get_name()).into())
    }
}

pub fn create_engine() -> Engine {
    // TODO: redirects print and debug to the log
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_string_size(10_000);
    engine.set_max_array_size(1_000);
    engine.set_max_map_size(1_000);
    engine.disable_symbol("eval");

    engine
        .register_type_with_name::<Loader>("Loader")
        .register_fn("patch_constant", Loader::patch_constant);
    engine
}
