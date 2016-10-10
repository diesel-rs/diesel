#![feature(rustc_private, plugin_registrar)]

extern crate syntax;
extern crate rustc_plugin;

#[plugin_registrar]
pub fn register(_reg: &mut rustc_plugin::Registry) {
}
