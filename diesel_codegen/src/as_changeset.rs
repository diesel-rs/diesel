use syn;
use quote;

use constants::custom_attrs::CHANGESET_OPTIONS;
use constants::custom_attr_options::TREAT_NONE_AS_NULL;
use constants::custom_derives::AS_CHANGESET;
use constants::syntax::{ID, LIFETIME_A};
use constants::values::TRUE;
use model::Model;
use util::attr_with_name;

pub fn derive_as_changeset(item: syn::MacroInput) -> quote::Tokens {
    let treat_none_as_null = format!("{}", treat_none_as_null(&item.attrs));
    let model = t!(Model::from_item(&item, AS_CHANGESET));

    let struct_name = &model.name;
    let table_name = model.table_name();
    let struct_ty = &model.ty;
    let mut lifetimes = item.generics.lifetimes;
    let attrs = model.attrs.into_iter()
        .filter(|a| a.column_name != Some(syn::Ident::new(ID)))
        .collect::<Vec<_>>();

    if lifetimes.is_empty() {
        lifetimes.push(syn::LifetimeDef::new(LIFETIME_A));
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
    let options_attr = match attr_with_name(attrs, CHANGESET_OPTIONS) {
        Some(attr) => attr,
        None => return false,
    };

    let usage_err = || panic!(r#"`#[{}]` must be in the form \
        `#[{}({} = "{}")]`"#, CHANGESET_OPTIONS,
        CHANGESET_OPTIONS, TREAT_NONE_AS_NULL, TRUE);

    match options_attr.value {
        syn::MetaItem::List(_, ref values) => {
            if values.len() != 1 {
                usage_err();
            }
            match values[0] {
                syn::MetaItem::NameValue(ref name, syn::Lit::Str(ref value, _))
                    if name == TREAT_NONE_AS_NULL => value == TRUE,
                _ => usage_err(),
            }
        }
        _ => usage_err(),
    }
}
