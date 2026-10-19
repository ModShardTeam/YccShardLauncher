use std::{collections::HashMap, range::Range, sync::OnceLock};

use crate::pe::addr;

pub type VarAddress = addr::Va;
pub type FnAddress = Range<addr::Va>;

pub static VARS: OnceLock<HashMap<String, VarAddress>> = OnceLock::new();
pub static FNS: OnceLock<HashMap<String, FnAddress>> = OnceLock::new();

pub fn get_function_address(func_name: &str) -> Option<&FnAddress> {
    FNS.get()?.get(func_name)
}

pub fn is_function_name_valid(func_name: &str) -> bool {
    get_function_address(func_name).is_some()
}
