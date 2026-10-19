use rhai::EvalAltResult;

use crate::{
    hooks::frame,
    runtime::{instance, symbols},
};

pub fn set_var_float(var_name: &str, value: f64) -> Result<(), Box<EvalAltResult>> {
    let Some(current_frame) = frame::current() else {
        return Err("set_var can only be called inside a prefix/postfix".into());
    };

    let Some(vars) = symbols::VARS.get() else {
        return Err("VARS table not initialized yet".into());
    };
    if let Err(e) = unsafe {
        instance::set_var(
            vars,
            current_frame.this,
            var_name,
            &instance::Variant::Real(value),
        )
    } {
        return Err(format!("cant set {:?}: {e}", var_name).into());
    }
    Ok(())
}

pub fn set_var_int(var_name: &str, value: i64) -> Result<(), Box<EvalAltResult>> {
    set_var_float(var_name, value as f64)
}
