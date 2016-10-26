use syn;
use quote;

use constants::{custom_attrs, custom_attr_options, custom_derives, syntax, values};
use model::Model;
use util::attr_with_name;

pub fn derive_as_changeset(item: syn::MacroInput) -> quote::Tokens {
    let treat_none_as_null = format!("{}", treat_none_as_null(&item.attrs));
    let model = t!(Model::from_item(&item, custom_derives::AS_CHANGESET));

    let struct_name = &model.name;
    let table_name = model.table_name();
    let struct_ty = &model.ty;
    let mut lifetimes = item.generics.lifetimes;
    let attrs = model.attrs.into_iter()
        .filter(|a| a.column_name != Some(syn::Ident::new(syntax::ID)))
        .collect::<Vec<_>>();

    if lifetimes.is_empty() {
        lifetimes.push(syn::LifetimeDef::new(syntax::LIFETIME_A));
    }

    quote!(AsChangeset! {
        (
            struct_name = #struct_name,
            table_name = #table_name,
            treat_none_as_null = #treat_none_as_null,
            struct_ty = #struct_ty,
            lifetimes = (#(lifetimes),*),
        ),
        fields = [#(attrs)*],
    })
}

fn treat_none_as_null(attrs: &[syn::Attribute]) -> bool {
    let options_attr = match attr_with_name(attrs, custom_attrs::CHANGESET_OPTIONS) {
        Some(attr) => attr,
        None => return false,
    };

    let usage_err = || panic!(r#"`#[{}]` must be in the form \
        `#[{}({} = "{}")]`"#, custom_attrs::CHANGESET_OPTIONS,
        custom_attrs::CHANGESET_OPTIONS, custom_attr_options::TREAT_NONE_AS_NULL, values::TRUE);

    match options_attr.value {
        syn::MetaItem::List(_, ref values) => {
            if values.len() != 1 {
                usage_err();
            }
            match values[0] {
                syn::MetaItem::NameValue(ref name, syn::Lit::Str(ref value, _))
                    if name == custom_attr_options::TREAT_NONE_AS_NULL => value == values::TRUE,
                _ => usage_err(),
            }
        }
        _ => usage_err(),
    }
}
