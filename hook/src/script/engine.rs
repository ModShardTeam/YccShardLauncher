use rhai::Engine;

use crate::script::{api, loader};

pub fn create_rhai_engine() -> Engine {
    // TODO: redirects print and debug to the log
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_string_size(10_000);
    engine.set_max_array_size(1_000);
    engine.set_max_map_size(1_000);
    engine.disable_symbol("eval");

    engine
        .register_type_with_name::<loader::Loader>("Loader")
        .register_type_with_name::<loader::ScriptLoader>("ScriptLoader")
        .register_fn("load_script", loader::Loader::load_script)
        .register_fn("replace_constant", loader::ScriptLoader::replace_constant)
        .register_fn("add_postfix", loader::ScriptLoader::add_postfix)
        .register_fn("add_prefix", loader::ScriptLoader::add_prefix)
        .register_fn("save", loader::ScriptLoader::save)
        .register_fn("set_var", api::set_var_float)
        .register_fn("set_var", api::set_var_int);
    engine
}
