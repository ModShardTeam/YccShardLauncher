use crate::{patch, symbols, utils};
use std::ffi::c_void;

pub fn my_step(this: *mut c_void, _other: *mut c_void) {
    let Some(vars) = symbols::VARS.get() else {
        log!("VARS table not initialized yet");
        return;
    };
    if let Err(e) = unsafe { utils::set_var(vars, this, "HP", &utils::Variant::Real(50.0)) } {
        // TODO: ignore IdNotReady, but it's a non-issue currently, IdNotReady is not seen in the log
        log!("{:#?}", e)
    }
}

pub fn my_constant_patch() {
    match patch::find_imm64("gml_Object_o_player_Step_0", 250.0) {
        Ok(a) => {
            unsafe {
                if let Err(e) = patch::write_f64(a, 10.0) {
                    log!("Error overwriting a constant: {}", e);
                }
            };
        }
        Err(e) => log!("xp patch disabled: {e}"),
    }
}
