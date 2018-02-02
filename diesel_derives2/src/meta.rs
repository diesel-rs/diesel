use proc_macro2::Span;
use syn;
use syn::spanned::Spanned;

use diagnostic_shim::*;

pub struct MetaItem {
    // Due to https://github.com/rust-lang/rust/issues/47941
    // we can only ever get the span of the #, which is better than nothing
    pound_span: Span,
    meta: syn::Meta,
}

impl MetaItem {
    pub fn with_name<'a>(attrs: &[syn::Attribute], name: &'a str) -> Option<Self> {
        attrs
            .iter()
            .filter_map(|attr| attr.interpret_meta().map(|m| (attr.pound_token.0[0], m)))
            .find(|&(_, ref m)| m.name() == name)
            .map(|(pound_span, meta)| Self { pound_span, meta })
    }

    pub fn nested_item<'a>(&self, name: &'a str) -> Result<Self, Diagnostic> {
        self.nested().and_then(|mut i| {
            i.nth(0).ok_or_else(|| {
                self.span()
                    .error(format!("Missing required option {}", name))
            })
        })
    }

    pub fn expect_bool_value(&self) -> bool {
        match self.str_value().as_ref().map(|s| s.as_str()) {
            Ok("true") => true,
            Ok("false") => false,
            _ => {
                self.span()
                    .error(format!(
                        "`{0}` must be in the form `{0} = \"true\"`",
                        self.name()
                    ))
                    .emit();
                false
            }
        }
    }

    pub fn expect_ident_value(&self) -> syn::Ident {
        let maybe_attr = self.nested().ok().and_then(|mut n| n.nth(0));
        let maybe_word = maybe_attr.as_ref().and_then(|m| m.word().ok());
        match maybe_word {
            Some(x) => {
                self.span()
                    .warning(format!(
                        "The form `{0}(value)` is deprecated. Use `{0} = \"value\"` instead",
                        self.name(),
                    ))
                    .emit();
                x
            }
            _ => syn::Ident::new(
                &self.expect_str_value(),
                self.value_span().resolved_at(Span::call_site()),
            ),
        }
    }

    pub fn expect_word(self) -> syn::Ident {
        self.word().unwrap_or_else(|e| {
            e.emit();
            self.name()
        })
    }

    pub fn word(&self) -> Result<syn::Ident, Diagnostic> {
        use syn::Meta::*;

        match self.meta {
            Word(mut x) => {
                x.span = self.span_or_pound_token(x.span);
                Ok(x)
            }
            _ => {
                let meta = &self.meta;
                Err(self.span().error(format!(
                    "Expected `{}` found `{}`",
                    self.name(),
                    quote!(#meta)
                )))
            }
        }
    }

    pub fn nested(&self) -> Result<Nested, Diagnostic> {
        use syn::Meta::*;

        match self.meta {
            List(ref list) => Ok(Nested(list.nested.iter(), self.pound_span)),
            _ => Err(self.span()
                .error(format!("`{0}` must be in the form `{0}(...)`", self.name()))),
        }
    }

    fn expect_str_value(&self) -> String {
        self.str_value().unwrap_or_else(|e| {
            e.emit();
            self.name().to_string()
        })
    }

    fn name(&self) -> syn::Ident {
        self.meta.name()
    }

    fn str_value(&self) -> Result<String, Diagnostic> {
        use syn::Meta::*;
        use syn::MetaNameValue;
        use syn::Lit::*;

        match self.meta {
            NameValue(MetaNameValue {
                lit: Str(ref s), ..
            }) => Ok(s.value()),
            _ => Err(self.span().error(format!(
                "`{0}` must be in the form `{0} = \"value\"`",
                self.name()
            ))),
        }
    }

    fn value_span(&self) -> Span {
        use syn::Meta::*;

        let s = match self.meta {
            Word(ident) => ident.span,
            List(ref meta) => meta.nested.span(),
            NameValue(ref meta) => meta.lit.span(),
        };
        self.span_or_pound_token(s)
    }

    fn span(&self) -> Span {
        self.span_or_pound_token(self.meta.span())
    }

    /// If the given span is affected by
    /// https://github.com/rust-lang/rust/issues/47941,
    /// returns the span of the pound token
    fn span_or_pound_token(&self, span: Span) -> Span {
        let bad_span_debug = "Span(Span { lo: BytePos(0), hi: BytePos(0), ctxt: #0 })";
        if format!("{:?}", span) == bad_span_debug {
            self.pound_span
        } else {
            span
        }
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)] // https://github.com/rust-lang-nursery/rustfmt/issues/2392
pub struct Nested<'a>(syn::punctuated::Iter<'a, syn::NestedMeta, Token![,]>, Span);

impl<'a> Iterator for Nested<'a> {
    type Item = MetaItem;

    fn next(&mut self) -> Option<Self::Item> {
        use syn::NestedMeta::*;

        match self.0.next() {
            Some(&Meta(ref item)) => Some(MetaItem {
                pound_span: self.1,
                meta: item.clone(),
            }),
            Some(_) => self.next(),
            None => None,
        }
    }
}
