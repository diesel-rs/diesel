use syn;
use quote;

use model::{Model, infer_association_name};
use util::str_value_of_meta_item;

pub fn derive_associations(input: syn::MacroInput) -> quote::Tokens {
    let mut derived_associations = Vec::new();
    let model = t!(Model::from_item(&input, "Associations"));

    for attr in &input.attrs {
        if attr.name() == "has_many" {
            let options = t!(build_association_options(attr, "has_many"));
            derived_associations.push(expand_has_many(&model, options))
        }

        if attr.name() == "belongs_to" {
            let options = t!(build_association_options(attr, "belongs_to"));
            derived_associations.push(expand_belongs_to(&model, options))
        }
    }

    quote!(#(derived_associations)*)
}

fn expand_belongs_to(model: &Model, options: AssociationOptions) -> quote::Tokens {
    let parent_struct = options.name;
    let struct_name = &model.name;

    let foreign_key_name = options.foreign_key_name.unwrap_or_else(||
        to_foreign_key(&parent_struct.as_ref()));
    let child_table_name = model.table_name();
    let fields = &model.attrs;

    quote!(BelongsTo! {
        (
            struct_name = #struct_name,
            parent_struct = #parent_struct,
            foreign_key_name = #foreign_key_name,
            child_table_name = #child_table_name,
        ),
        fields = [#(fields)*],
    })
}

fn expand_has_many(model: &Model, options: AssociationOptions) -> quote::Tokens {
    let parent_table_name = model.table_name();
    let child_table_name = options.name;
    let foreign_key_name = options.foreign_key_name.unwrap_or_else(||
        to_foreign_key(&model.name.as_ref()));
    let fields = &model.attrs;

    quote!(HasMany! {
        (
            parent_table_name = #parent_table_name,
            child_table = #child_table_name::table,
            foreign_key = #child_table_name::#foreign_key_name,
        ),
        fields = [#(fields)*],
    })
}

struct AssociationOptions {
    name: syn::Ident,
    foreign_key_name: Option<syn::Ident>,
}

fn build_association_options(
    attr: &syn::Attribute,
    association_kind: &str,
) -> Result<AssociationOptions, String> {
    let usage_error = Err(format!(
            "`#[{}]` must be in the form `#[{}(table_name, option=value)]`",
            association_kind, association_kind));
    match attr.value {
        syn::MetaItem::List(_, ref options) if options.len() >= 1 => {
            let association_name = match options[0] {
                syn::MetaItem::Word(ref name) => name.clone(),
                _ => return usage_error,
            };
            let foreign_key_name = options.iter().find(|a| a.name() == "foreign_key")
                .map(|a| str_value_of_meta_item(a, "foreign_key"))
                .map(syn::Ident::new);

            Ok(AssociationOptions {
                name: association_name,
                foreign_key_name: foreign_key_name,
            })
        }
        _ => usage_error,
    }
}

fn to_foreign_key(model_name: &str) -> syn::Ident {
    let lower_cased = infer_association_name(model_name);
    syn::Ident::new(format!("{}_id", &lower_cased))
}

#[test]
fn to_foreign_key_properly_handles_underscores() {
    assert_eq!(syn::Ident::new("foo_bar_id"), to_foreign_key("FooBar"));
    assert_eq!(syn::Ident::new("foo_bar_baz_id"), to_foreign_key("FooBarBaz"));
}
