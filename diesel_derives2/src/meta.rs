use syn;

pub struct MetaItem {
    meta: syn::Meta,
}

impl MetaItem {
    pub fn with_name<'a>(
        attrs: &[syn::Attribute],
        name: &'a str,
    ) -> Result<Self, MissingOption<'a>> {
        attrs
            .iter()
            .filter_map(syn::Attribute::interpret_meta)
            .find(|m| m.name() == name)
            .map(|meta| Self { meta })
            .ok_or(MissingOption { name })
    }

    pub fn nested_item<'a>(&self, name: &'a str) -> Result<Self, MissingOption<'a>> {
        self.expect_nested().nth(0).ok_or(MissingOption { name })
    }

    pub fn expect_bool_value(&self) -> bool {
        match self.str_value().as_ref().map(|s| s.as_str()) {
            Some("true") => true,
            Some("false") => false,
            _ => panic!("`{0}` must be in the form `{0} = \"true\"`", self.name()),
        }
    }

    pub fn expect_ident_value(&self) -> syn::Ident {
        let maybe_attr = self.nested().and_then(|mut n| n.nth(0));
        let maybe_word = maybe_attr.as_ref().and_then(Self::word);
        match maybe_word {
            Some(x) => {
                println!(
                    "The form `{0}(value)` is deprecated. Use `{0} = \"value\"` instead",
                    self.name()
                );
                x.clone()
            }
            _ => self.expect_str_value().into(),
        }
    }

    pub fn expect_word(&self) -> &syn::Ident {
        self.word().unwrap_or_else(|| {
            let meta = &self.meta;
            panic!("Expected `{}` found `{}`", self.name(), quote!(#meta))
        })
    }

    pub fn word(&self) -> Option<&syn::Ident> {
        use syn::Meta::*;

        match self.meta {
            Word(ref x) => Some(x),
            _ => None,
        }
    }

    pub fn expect_nested(&self) -> Nested {
        self.nested()
            .unwrap_or_else(|| panic!("`{0}` must be in the form `{0}(...)`", self.name()))
    }

    pub fn nested(&self) -> Option<Nested> {
        use syn::Meta::*;

        match self.meta {
            List(ref list) => Some(Nested(list.nested.iter())),
            _ => None,
        }
    }

    fn expect_str_value(&self) -> String {
        self.str_value()
            .unwrap_or_else(|| panic!("`{0}` must be in the form `{0} = \"value\"`", self.name()))
    }

    fn name(&self) -> syn::Ident {
        self.meta.name()
    }

    fn str_value(&self) -> Option<String> {
        use syn::Meta::*;
        use syn::MetaNameValue;
        use syn::Lit::*;

        match self.meta {
            NameValue(MetaNameValue {
                lit: Str(ref s), ..
            }) => Some(s.value()),
            _ => None,
        }
    }
}

pub struct MissingOption<'a> {
    name: &'a str,
}

pub trait RequiredOption<T> {
    fn required(self) -> T;
}

impl<'a> RequiredOption<MetaItem> for Result<MetaItem, MissingOption<'a>> {
    fn required(self) -> MetaItem {
        self.unwrap_or_else(|e| panic!("Missing required option {}", e.name))
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
