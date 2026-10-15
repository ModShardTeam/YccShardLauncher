use std::ffi::c_void;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Real = 0,
    String = 1,
    Undefined = 5,
    Bool = 13,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct YYCVal {
    pub value: u64,
    pub flags: u32,
    pub kind: u32,
}

pub type VarGetter = unsafe extern "C" fn(*mut c_void, usize) -> *mut YYCVal;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct YYCVar {
    pub name_ptr: usize,
    pub id: i32,
    pub zero: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct YYCFn {
    pub name_ptr: usize,
    pub func: usize,
    pub slot_ptr: usize,
}

const _: () = assert!(size_of::<YYCVal>() == 16);
const _: () = assert!(size_of::<YYCVar>() == 16);
const _: () = assert!(size_of::<YYCFn>() == 24);
