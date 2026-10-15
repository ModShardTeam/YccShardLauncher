use std::{collections::HashMap, ffi::c_void};

use crate::{
    pe_structs::Va,
    yyc_structs::{Kind, VarGetter, YYCVal},
};

pub enum Variant {
    Real(f64),
    Undefined,
    Bool(bool),
}

impl From<&Variant> for YYCVal {
    fn from(var: &Variant) -> Self {
        match var {
            Variant::Real(v) => YYCVal {
                value: v.to_bits(),
                flags: 0,
                kind: Kind::Real as u32,
            },
            Variant::Undefined => YYCVal {
                value: 0u64,
                flags: 0,
                kind: Kind::Undefined as u32,
            },
            Variant::Bool(v) => YYCVal {
                value: if *v {
                    1.0f64.to_bits()
                } else {
                    0.0f64.to_bits()
                },
                flags: 0,
                kind: Kind::Bool as u32,
            },
        }
    }
}

const VAR_MUT_GETTER_VTABLE_IDX: usize = 2;

#[derive(Debug)]
pub enum VarRejection {
    MissingVar,
    IdNotReady,
    NullObject,
    NullVar,
    IncompatibleKind,
}

unsafe fn var_id(vars: &HashMap<String, Va>, name: &str) -> Result<usize, VarRejection> {
    let Some(&id_ptr) = vars.get(name) else {
        return Err(VarRejection::MissingVar);
    };

    // TODO: the read is not atomic
    // Before the start of the game id = -1
    // Ids shouldnt be read before being initialized
    let id = unsafe { *(id_ptr.as_ptr::<i32>()) };
    if id > 0 {
        Ok(id as usize)
    } else {
        Err(VarRejection::IdNotReady)
    }
}

/// # Safety
///
/// This pointer should point to a live game instance and vars should come from var scanner
pub unsafe fn get_var(
    vars: &HashMap<String, Va>,
    this: *mut c_void,
    name: &str,
) -> Result<*mut YYCVal, VarRejection> {
    if this.is_null() {
        return Err(VarRejection::NullObject);
    }
    let id = unsafe { var_id(vars, name) }?;
    let vtable = unsafe { *(this as *const *const usize) };
    let f: VarGetter = unsafe { std::mem::transmute(*vtable.add(VAR_MUT_GETTER_VTABLE_IDX)) };
    Ok(unsafe { f(this, id) })
}

/// # Safety
///
/// This pointer should point to a live game instance and vars should come from var scanner
pub unsafe fn set_var(
    vars: &HashMap<String, Va>,
    this: *mut c_void,
    name: &str,
    val: &Variant,
) -> Result<(), VarRejection> {
    let p = unsafe { get_var(vars, this, name) }?;
    if p.is_null() {
        return Err(VarRejection::NullVar);
    }
    let safe_p = unsafe { &mut *p };
    let yyc_val: YYCVal = YYCVal::from(val);
    if safe_p.kind != yyc_val.kind {
        return Err(VarRejection::IncompatibleKind);
    }
    safe_p.value = yyc_val.value;
    Ok(())
}
