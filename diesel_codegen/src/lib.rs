#![feature(rustc_private, plugin_registrar)]

extern crate diesel_codegen_syntex;
extern crate syntax;
extern crate rustc_plugin;

use diesel_codegen_syntex::*;

#[plugin_registrar]
pub fn register(reg: &mut rustc_plugin::Registry) {
    use syntax::parse::token::intern;
    use syntax::ext::base::MultiDecorator;
    reg.register_syntax_extension(
        intern("derive_Queryable"),
        MultiDecorator(Box::new(queryable::expand_derive_queryable))
    );
    reg.register_syntax_extension(
        intern("derive_Identifiable"),
        MultiDecorator(Box::new(identifiable::expand_derive_identifiable))
    );
    reg.register_syntax_extension(
        intern("insertable_into"),
        MultiDecorator(Box::new(insertable::expand_insert))
    );
    reg.register_syntax_extension(
        intern("changeset_for"),
        MultiDecorator(Box::new(update::expand_changeset_for)),
    );
    reg.register_syntax_extension(
        intern("has_many"),
        MultiDecorator(Box::new(associations::expand_has_many))
    );
    reg.register_syntax_extension(
        intern("belongs_to"),
        MultiDecorator(Box::new(associations::expand_belongs_to))
    );
    reg.register_macro("embed_migrations", migrations::expand_embed_migrations);
    reg.register_macro("infer_table_from_schema", schema_inference::expand_load_table);
    reg.register_macro("infer_schema", schema_inference::expand_infer_schema);
}
