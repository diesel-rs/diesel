#![feature(rustc_private, plugin, plugin_registrar)]
#![plugin(quasi_macros)]

extern crate aster;
extern crate quasi;

extern crate syntax;
extern crate rustc_plugin;

mod associations;
mod attr;
mod insertable;
mod model;
mod queriable;

#[plugin_registrar]
pub fn register(reg: &mut rustc_plugin::Registry) {
    use syntax::parse::token::intern;
    use syntax::ext::base::MultiDecorator;
    reg.register_syntax_extension(
        intern("derive_Queriable"),
        MultiDecorator(Box::new(queriable::expand_derive_queriable))
    );
    reg.register_syntax_extension(
        intern("insertable_into"),
        MultiDecorator(Box::new(insertable::expand_insert))
    );
    reg.register_syntax_extension(
        intern("has_many"),
        MultiDecorator(Box::new(associations::expand_has_many))
    );
    reg.register_syntax_extension(
        intern("belongs_to"),
        MultiDecorator(Box::new(associations::expand_belongs_to))
    );
}
