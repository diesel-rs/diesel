use syn;
use quote;

use model::Model;
use util::attr_with_name;

pub fn derive_as_changeset(item: syn::DeriveInput) -> quote::Tokens {
    let treat_none_as_null = format!("{}", treat_none_as_null(&item.attrs));
    let model = t!(Model::from_item(&item, "AsChangeset"));

    let struct_name = &model.name;
    let table_name = model.table_name();
    let struct_ty = &model.ty;
    let mut lifetimes = item.generics.lifetimes;
    let attrs = model
        .attrs
        .as_slice()
        .iter()
        .filter(|a| !model.primary_key_names.contains(a.column_name()))
        .collect::<Vec<_>>();

    if attrs.is_empty() {
        panic!(
            "Deriving `AsChangeset` on a structure that only contains the primary key isn't \
             supported. If you want to change the primary key of a row, you should do so with \
             `.set(table::id.eq(new_id))`. `AsChangeset` never changes the primary key of a row."
        );
    }

    if lifetimes.is_empty() {
        lifetimes.push(syn::LifetimeDef::new("'a"));
    }

    quote!(impl_AsChangeset! {
        (
            struct_name = #struct_name,
            table_name = #table_name,
            treat_none_as_null = #treat_none_as_null,
            struct_ty = #struct_ty,
            lifetimes = (#(#lifetimes),*),
        ),
        fields = [#(#attrs)*],
    })
}

fn treat_none_as_null(attrs: &[syn::Attribute]) -> bool {
    let options_attr = match attr_with_name(attrs, "changeset_options") {
        Some(attr) => attr,
        None => return false,
    };

    let usage_err = || {
        panic!(
            r#"`#[changeset_options]` must be in the form \
        `#[changeset_options(treat_none_as_null = "true")]`"#
        )
    };

    match options_attr.value {
        syn::MetaItem::List(_, ref values) => {
            if values.len() != 1 {
                usage_err();
            }
            match values[0] {
                syn::NestedMetaItem::MetaItem(
                    syn::MetaItem::NameValue(ref name, syn::Lit::Str(ref value, _)),
                ) if name == "treat_none_as_null" =>
                {
                    value == "true"
                }
                _ => usage_err(),
            }
        }
        _ => usage_err(),
    }
}
