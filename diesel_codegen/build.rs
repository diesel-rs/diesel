#[cfg(feature = "with-syntex")]
mod inner {
    extern crate syntex;
    extern crate syntex_syntax as syntax;

    use std::env;
    use std::path::Path;

    use self::syntax::codemap::Span;
    use self::syntax::ext::base::{self, ExtCtxt};
    use self::syntax::tokenstream::TokenTree;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut registry = syntex::Registry::new();

        macro_rules! register_quote_macro {
            ($macro_name: ident, $name: ident) => {
                fn $name<'cx>(
                    cx: &'cx mut ExtCtxt,
                    sp: Span,
                    tts: &[tokenstream::TokenTree],
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
    }
}

#[cfg(not(feature = "with-syntex"))]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
