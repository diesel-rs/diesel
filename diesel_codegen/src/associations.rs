use syn;
use quote;

use model::{Model, infer_association_name};
use util::{str_value_of_meta_item, wrap_item_in_const};

pub fn derive_associations(input: syn::MacroInput) -> quote::Tokens {
    let model = t!(Model::from_item(&input, "Associations"));

    let derived_associations = input.attrs.as_slice()
        .iter()
        .filter_map(|attr| {
            if attr.name() == "belongs_to" {
                Some(t!(build_association_options(attr, "belongs_to")))
            } else {
                None
            }
        }).map(|option|{
            expand_belongs_to(&model, &option)
        }).collect::<Vec<_>>();

    let derived_associations = quote!(#(#derived_associations)*);

    let model_name_uppercase = model.name.as_ref().to_uppercase();
    let dummy_const = format!("_IMPL_ASSOCIATIONS_FOR_{}", model_name_uppercase).into();
    wrap_item_in_const(dummy_const, derived_associations)
}

fn expand_belongs_to(model: &Model, options: &AssociationOptions) -> quote::Tokens
{
    let parent_struct = &options.name;
    let struct_name = &model.name;

    let foreign_key_name = options.foreign_key_name.clone()
        .unwrap_or_else(|| to_foreign_key(parent_struct.as_ref()));
    let child_table_name = model.table_name();
    let fields = model.attrs.as_slice();

    let foreign_key_name = &foreign_key_name;
    let child_table_name = &child_table_name;
    let foreign_key_attr = fields.iter()
        .find(|attr| attr.column_name.as_ref() == Some(foreign_key_name)).unwrap();
    let foreign_key_ty = &foreign_key_attr.ty;
    // we need to special case foreign keys on with an Option type
    // to allow self referencing joins
    let (foreign_key, foreign_key_ty) = match *foreign_key_ty {
        syn::Ty::Path(None, ref p) if p.segments[0].ident == "Option" => {
            let segment = &p.segments[0];
            let t = match segment.parameters {
                syn::PathParameters::AngleBracketed(ref p) => {
                    p.types[0].clone()
                }
                syn::PathParameters::Parenthesized(_) => unreachable!()
            };
            (quote!(self.#foreign_key_name.as_ref()), t)
        }
        ref t => (quote!(Some(&self.#foreign_key_name)), t.clone())
    };

    quote! {
        impl diesel::associations::BelongsTo<#parent_struct> for #struct_name
        {
            type ForeignKey = #foreign_key_ty;
            type ForeignKeyColumn = #child_table_name::#foreign_key_name;

            fn foreign_key(&self) -> Option<&#foreign_key_ty> {
                #foreign_key
            }

            fn foreign_key_column() -> #child_table_name::#foreign_key_name {
                #child_table_name::#foreign_key_name
            }
        }
    }
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
                syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) => name.clone(),
                _ => return usage_error,
            };
            let foreign_key_name = options.iter()
                .filter_map(|o| match *o {
                    syn::NestedMetaItem::MetaItem(ref mi) => Some(mi),
                    _ => None
                })
                .find(|a| a.name() == "foreign_key")
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
