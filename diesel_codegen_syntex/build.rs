extern crate syntex;
extern crate syntex_syntax as syntax;

use std::env;
use std::path::Path;
use std::thread;

use self::syntax::codemap::Span;
use self::syntax::ext::base::{self, ExtCtxt};
use self::syntax::tokenstream::TokenTree;

fn main() {
    with_extra_stack(move || {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut registry = syntex::Registry::new();

        macro_rules! register_quote_macro {
            ($macro_name: ident, $name: ident) => {
                fn $name<'cx>(
                    cx: &'cx mut ExtCtxt,
                    sp: Span,
                    tts: &[TokenTree],
                ) -> Box<base::MacResult + 'cx> {
                    syntax::ext::quote::$name(cx, sp, tts)
                }

                registry.add_macro(stringify!($macro_name), $name);
            }
        }

        register_quote_macro!(quote_ty, expand_quote_ty);
        register_quote_macro!(quote_item, expand_quote_item);
        register_quote_macro!(quote_tokens, expand_quote_tokens);
        register_quote_macro!(quote_expr, expand_quote_expr);

        let src = Path::new("src/lib.in.rs");
        let dst = Path::new(&out_dir).join("lib.rs");

        registry.expand("", &src, &dst).unwrap();
    });
}

fn with_extra_stack<F: FnOnce() + Send + 'static>(f: F) {
    env::set_var("RUST_MIN_STACK", "16777216"); // 16MB
    thread::spawn(f).join().unwrap();
}
