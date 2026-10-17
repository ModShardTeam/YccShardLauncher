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
