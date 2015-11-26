#![feature(rustc_private, plugin, plugin_registrar)]
#![plugin(quasi_macros)]

extern crate aster;
extern crate quasi;

extern crate syntax;
extern crate rustc;

mod queriable;

#[plugin_registrar]
pub fn register(reg: &mut rustc::plugin::Registry) {
    reg.register_syntax_extension(
        syntax::parse::token::intern("derive_Queriable"),
        syntax::ext::base::MultiDecorator(
            Box::new(queriable::expand_derive_queriable)));
}
