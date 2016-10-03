#![feature(rustc_private, plugin_registrar)]

extern crate diesel_codegen_syntex;
extern crate syntax;
extern crate rustc_plugin;

use diesel_codegen_syntex::*;

#[plugin_registrar]
pub fn register(reg: &mut rustc_plugin::Registry) {
    reg.register_macro("embed_migrations", migrations::expand_embed_migrations);
    reg.register_macro("infer_table_from_schema", schema_inference::expand_load_table);
    reg.register_macro("infer_schema", schema_inference::expand_infer_schema);
}
