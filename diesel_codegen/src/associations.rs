use syn;
use quote;

use constants::{custom_attrs, custom_attr_options, custom_derives};
use model::{Model, infer_association_name};
use util::str_value_of_meta_item;

pub fn derive_associations(input: syn::MacroInput) -> quote::Tokens {
    let mut derived_associations = Vec::new();
    let model = t!(Model::from_item(&input, custom_derives::ASSOCIATIONS));

    for attr in &input.attrs {
        if attr.name() == custom_attrs::HAS_MANY {
            let options = t!(build_association_options(attr, custom_attrs::HAS_MANY));
            derived_associations.push(expand_has_many(&model, options))
        }

        if attr.name() == custom_attrs::BELONGS_TO {
            let options = t!(build_association_options(attr, custom_attrs::BELONGS_TO));
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
            "`#[{}]` must be in the form `#[{}({}, option=value)]`",
            association_kind, association_kind, custom_attr_options::TABLE_NAME));
    match attr.value {
        syn::MetaItem::List(_, ref options) if options.len() >= 1 => {
            let association_name = match options[0] {
                syn::MetaItem::Word(ref name) => name.clone(),
                _ => return usage_error,
            };
            let foreign_key_name = options.iter()
                .find(|a| a.name() == custom_attr_options::FOREIGN_KEY)
                .map(|a| str_value_of_meta_item(a, custom_attr_options::FOREIGN_KEY))
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
