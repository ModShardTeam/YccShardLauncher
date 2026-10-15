use std::collections::HashMap;
use std::sync::OnceLock;

use crate::pe_structs;

pub static VARS: OnceLock<HashMap<String, pe_structs::VarAddress>> = OnceLock::new();
pub static FNS: OnceLock<HashMap<String, pe_structs::FnAddress>> = OnceLock::new();

pub fn get_function_address(func_name: &str) -> Option<&pe_structs::FnAddress> {
    FNS.get()?.get(func_name)
}
