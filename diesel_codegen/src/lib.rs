#![cfg_attr(not(feature = "with-syntex"), feature(rustc_private, plugin_registrar, quote))]
#![deny(warnings)]

#[macro_use] extern crate diesel;

#[cfg(feature = "with-syntex")]
extern crate syntex;

#[cfg(feature = "with-syntex")]
extern crate syntex_syntax as syntax;

#[cfg(not(feature = "with-syntex"))]
extern crate syntax;

#[cfg(not(feature = "with-syntex"))]
extern crate rustc_plugin;

#[cfg(feature = "with-syntex")]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

#[cfg(not(feature = "with-syntex"))]
include!("lib.in.rs");

mod util;

#[cfg(feature = "with-syntex")]
pub fn register(reg: &mut syntex::Registry) {
    reg.add_attr("feature(custom_derive)");
    reg.add_attr("feature(custom_attribute)");

    reg.add_decorator("derive_Queryable", queryable::expand_derive_queryable);
    reg.add_decorator("insertable_into", insertable::expand_insert);
    reg.add_decorator("changeset_for", update::expand_changeset_for);
    reg.add_decorator("has_many", associations::expand_has_many);
    reg.add_decorator("belongs_to", associations::expand_belongs_to);
    reg.add_macro("embed_migrations", migrations::expand_embed_migrations);
    reg.add_macro("infer_table_from_schema", schema_inference::expand_load_table);
    reg.add_macro("infer_schema", schema_inference::expand_infer_schema);

    reg.add_post_expansion_pass(util::strip_attributes);
}

#[cfg_attr(not(feature = "with-syntex"), plugin_registrar)]
#[cfg(not(feature = "with-syntex"))]
pub fn register(reg: &mut rustc_plugin::Registry) {
    use syntax::parse::token::intern;
    use syntax::ext::base::MultiDecorator;
    reg.register_syntax_extension(
        intern("derive_Queryable"),
        MultiDecorator(Box::new(queryable::expand_derive_queryable))
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
