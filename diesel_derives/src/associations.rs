use proc_macro2;
use syn;

use diagnostic_shim::*;
use meta::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;
    let tokens = MetaItem::all_with_name(&item.attrs, "belongs_to")
        .into_iter()
        .filter_map(
            |attr| match derive_belongs_to(&model, &item.generics, attr) {
                Ok(t) => Some(t),
                Err(e) => {
                    e.emit();
                    None
                }
            },
        );

    Ok(wrap_in_dummy_mod(
        model.dummy_mod_name("associations"),
        quote!(#(#tokens)*),
    ))
}

fn derive_belongs_to(
    model: &Model,
    generics: &syn::Generics,
    meta: MetaItem,
) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let AssociationOptions {
        parent_struct,
        foreign_key,
    } = AssociationOptions::from_meta(meta)?;
    let (_, ty_generics, _) = generics.split_for_impl();

    let foreign_key_field = model.find_column(&foreign_key)?;
    let struct_name = &model.name;
    let foreign_key_access = foreign_key_field.name.access();
    let foreign_key_ty = inner_of_option_ty(&foreign_key_field.ty);
    let table_name = model.table_name();

    let mut generics = generics.clone();

    // TODO: Remove this specialcasing as soon as we bump our minimal supported
    // rust version to >= 1.30.0 because this version will add
    // `impl<'a, T> From<&'a Option<T>> for Option<&'a T>` to the std-lib
    let foreign_key_expr = if is_option_ty(&foreign_key_field.ty) {
        quote!(self#foreign_key_access.as_ref().and_then(std::convert::Into::into))
    } else {
        quote!(std::convert::Into::into(&self#foreign_key_access))
    };

    generics.params.push(parse_quote!(__FK));
    {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!(__FK: std::hash::Hash + std::cmp::Eq));
        where_clause.predicates.push(
            parse_quote!(for<'__a> &'__a #foreign_key_ty: std::convert::Into<::std::option::Option<&'__a __FK>>),
        );
        where_clause.predicates.push(
            parse_quote!(for<'__a> &'__a #parent_struct: diesel::associations::Identifiable<Id = &'__a __FK>),
        );
    }

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics diesel::associations::BelongsTo<#parent_struct>
            for #struct_name #ty_generics
        #where_clause
        {
            type ForeignKey = __FK;
            type ForeignKeyColumn = #table_name::#foreign_key;

            fn foreign_key(&self) -> std::option::Option<&Self::ForeignKey> {
                #foreign_key_expr
            }

            fn foreign_key_column() -> Self::ForeignKeyColumn {
                #table_name::#foreign_key
            }
        }
    })
}

struct AssociationOptions {
    parent_struct: syn::Ident,
    foreign_key: syn::Ident,
}

impl AssociationOptions {
    fn from_meta(meta: MetaItem) -> Result<Self, Diagnostic> {
        let parent_struct = meta.nested()?
            .nth(0)
            .ok_or_else(|| meta.span())
            .and_then(|m| m.word().map_err(|_| m.span()))
            .map_err(|span| {
                span.error("Expected a struct name")
                    .help("e.g. `#[belongs_to(User)]`")
            })?;
        let foreign_key = meta.nested_item("foreign_key")
            .ok()
            .map(|i| i.ident_value())
            .unwrap_or_else(|| Ok(infer_foreign_key(&parent_struct)))?;

        let unrecognized_options = meta.nested()?.skip(1).filter(|n| n.name() != "foreign_key");
        for ignored in unrecognized_options {
            ignored
                .span()
                .warning(format!("Unrecognized option {}", ignored.name()))
                .emit();
        }

        Ok(Self {
            parent_struct,
            foreign_key,
        })
    }
}

fn infer_foreign_key(name: &syn::Ident) -> syn::Ident {
    let snake_case = camel_to_snake(&name.to_string());
    syn::Ident::new(&format!("{}_id", snake_case), name.span())
}
