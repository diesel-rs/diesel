use proc_macro2::{Ident, Span};
use syn;
use syn::fold::Fold;
use syn::spanned::Spanned;

use resolved_at_shim::*;
use util::*;

pub struct MetaItem {
    meta: syn::Meta,
}

impl MetaItem {
    pub fn all_with_name(attrs: &[syn::Attribute], name: &str) -> Vec<Self> {
        attrs
            .iter()
            .filter_map(|attr| {
                attr.interpret_meta()
                    .map(|m| FixSpan(attr.pound_token.spans[0]).fold_meta(m))
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
                ident: Ident::new(name, Span::call_site()),
                paren_token: Default::default(),
                nested: Default::default(),
            }),
        }
    }

    pub fn nested_item(&self, name: &str) -> Result<Option<Self>, Diagnostic> {
        self.nested().map(|mut i| i.find(|n| n.name() == name))
    }

    pub fn required_nested_item(&self, name: &str) -> Result<Self, Diagnostic> {
        self.nested_item(name)?.ok_or_else(|| {
            self.span()
                .error(format!("Missing required option `{}`", name))
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
            Word(ref x) => Ok(x.clone()),
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
            _ => Err(self
                .span()
                .error(format!("`{0}` must be in the form `{0}(...)`", self.name()))),
        }
    }

    pub fn name(&self) -> syn::Ident {
        self.meta.name()
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.nested()
            .map(|mut n| {
                n.any(|m| match m.word() {
                    Ok(word) => word == flag,
                    Err(_) => false,
                })
            })
            .unwrap_or_else(|e| {
                e.emit();
                false
            })
    }

    pub fn ty_value(&self) -> Result<syn::Type, Diagnostic> {
        let str = self.lit_str_value()?;
        str.parse()
            .map_err(|_| str.span().error("Invalid Rust type"))
    }

    pub fn expect_str_value(&self) -> String {
        self.str_value().unwrap_or_else(|e| {
            e.emit();
            self.name().to_string()
        })
    }

    pub fn str_value(&self) -> Result<String, Diagnostic> {
        self.lit_str_value().map(syn::LitStr::value)
    }

    fn lit_str_value(&self) -> Result<&syn::LitStr, Diagnostic> {
        use syn::Lit::*;

        match *self.lit_value()? {
            Str(ref s) => Ok(s),
            _ => Err(self.span().error(format!(
                "`{0}` must be in the form `{0} = \"value\"`",
                self.name()
            ))),
        }
    }

    pub fn expect_int_value(&self) -> u64 {
        self.int_value().emit_error().unwrap_or(0)
    }

    pub fn int_value(&self) -> Result<u64, Diagnostic> {
        use syn::Lit::*;

        let error = self.value_span().error("Expected a number");

        match *self.lit_value()? {
            Str(ref s) => s.value().parse().map_err(|_| error),
            Int(ref i) => Ok(i.value()),
            _ => Err(error),
        }
    }

    fn lit_value(&self) -> Result<&syn::Lit, Diagnostic> {
        use syn::Meta::*;

        match self.meta {
            NameValue(ref name_value) => Ok(&name_value.lit),
            _ => Err(self.span().error(format!(
                "`{0}` must be in the form `{0} = \"value\"`",
                self.name()
            ))),
        }
    }

    pub fn warn_if_other_options(&self, options: &[&str]) {
        let nested = match self.nested() {
            Ok(x) => x,
            Err(_) => return,
        };
        let unrecognized_options = nested.filter(|n| !options.iter().any(|&o| n.name() == o));
        for ignored in unrecognized_options {
            ignored
                .span()
                .warning(format!("Option {} has no effect", ignored.name()))
                .emit();
        }
    }

    fn value_span(&self) -> Span {
        use syn::Meta::*;

        match self.meta {
            Word(ref ident) => ident.span(),
            List(ref meta) => meta.nested.span(),
            NameValue(ref meta) => meta.lit.span(),
        }
    }

    pub fn span(&self) -> Span {
        self.meta.span()
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)] // https://github.com/rust-lang-nursery/rustfmt/issues/2392
pub struct Nested<'a>(syn::punctuated::Iter<'a, syn::NestedMeta>);

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
        fix_span(span, self.0)
    }
}
