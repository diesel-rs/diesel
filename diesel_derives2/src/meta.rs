use proc_macro2::Span;
use syn;
use syn::spanned::Spanned;
use syn::fold::Fold;

use diagnostic_shim::*;

pub struct MetaItem {
    meta: syn::Meta,
}

impl MetaItem {
    pub fn all_with_name(attrs: &[syn::Attribute], name: &str) -> Vec<Self> {
        attrs
            .iter()
            .filter_map(|attr| {
                attr.interpret_meta()
                    .map(|m| FixSpan(attr.pound_token.0[0]).fold_meta(m))
            })
            .filter(|m| m.name() == name)
            .map(|meta| Self { meta })
            .collect()
    }

    pub fn with_name(attrs: &[syn::Attribute], name: &str) -> Option<Self> {
        Self::all_with_name(attrs, name).pop()
    }

    pub fn empty(name: &str) -> Self {
        Self {
            meta: syn::Meta::List(syn::MetaList {
                ident: name.into(),
                paren_token: Default::default(),
                nested: Default::default(),
            }),
        }
    }

    pub fn nested_item(&self, name: &str) -> Result<Self, Diagnostic> {
        self.nested().and_then(|mut i| {
            i.find(|n| n.name() == name).ok_or_else(|| {
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
        self.ident_value().unwrap_or_else(|e| {
            e.emit();
            self.name()
        })
    }

    pub fn ident_value(&self) -> Result<syn::Ident, Diagnostic> {
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
                Ok(x)
            }
            _ => Ok(syn::Ident::new(
                &self.str_value()?,
                self.value_span().resolved_at(Span::call_site()),
            )),
        }
    }

    pub fn expect_word(&self) -> syn::Ident {
        self.word().unwrap_or_else(|e| {
            e.emit();
            self.name()
        })
    }

    pub fn word(&self) -> Result<syn::Ident, Diagnostic> {
        use syn::Meta::*;

        match self.meta {
            Word(x) => Ok(x),
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
            List(ref list) => Ok(Nested(list.nested.iter())),
            _ => Err(self.span()
                .error(format!("`{0}` must be in the form `{0}(...)`", self.name()))),
        }
    }

    pub fn name(&self) -> syn::Ident {
        self.meta.name()
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.nested()
            .map(|mut n| n.any(|m| m.expect_word() == flag))
            .unwrap_or_else(|e| {
                e.emit();
                false
            })
    }

    pub fn ty_value(&self) -> Result<syn::Type, Diagnostic> {
        let str = self.lit_str_value()?;
        str.parse().map_err(|_| str.span.error("Invalid Rust type"))
    }

    fn str_value(&self) -> Result<String, Diagnostic> {
        self.lit_str_value().map(syn::LitStr::value)
    }

    fn lit_str_value(&self) -> Result<&syn::LitStr, Diagnostic> {
        use syn::Meta::*;
        use syn::MetaNameValue;
        use syn::Lit::*;

        match self.meta {
            NameValue(MetaNameValue {
                lit: Str(ref s), ..
            }) => Ok(s),
            _ => Err(self.span().error(format!(
                "`{0}` must be in the form `{0} = \"value\"`",
                self.name()
            ))),
        }
    }

    fn value_span(&self) -> Span {
        use syn::Meta::*;

        match self.meta {
            Word(ident) => ident.span,
            List(ref meta) => meta.nested.span(),
            NameValue(ref meta) => meta.lit.span(),
        }
    }

    pub fn span(&self) -> Span {
        self.meta.span()
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)] // https://github.com/rust-lang-nursery/rustfmt/issues/2392
pub struct Nested<'a>(syn::punctuated::Iter<'a, syn::NestedMeta, Token![,]>);

impl<'a> Iterator for Nested<'a> {
    type Item = MetaItem;

    fn next(&mut self) -> Option<Self::Item> {
        use syn::NestedMeta::*;

        match self.0.next() {
            Some(&Meta(ref item)) => Some(MetaItem { meta: item.clone() }),
            Some(_) => self.next(),
            None => None,
        }
    }
}

/// If the given span is affected by
/// <https://github.com/rust-lang/rust/issues/47941>,
/// returns the span of the pound token
struct FixSpan(Span);

impl Fold for FixSpan {
    fn fold_span(&mut self, span: Span) -> Span {
        let bad_span_debug = "Span(Span { lo: BytePos(0), hi: BytePos(0), ctxt: #0 })";
        if format!("{:?}", span) == bad_span_debug {
            self.0
        } else {
            span
        }
    }
}
