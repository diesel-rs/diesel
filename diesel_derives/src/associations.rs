use quote;
use syn;

use diagnostic_shim::*;
use meta::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<quote::Tokens, Diagnostic> {
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
) -> Result<quote::Tokens, Diagnostic> {
    let AssociationOptions {
        parent_struct,
        foreign_key,
    } = AssociationOptions::from_meta(meta)?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let foreign_key_field = model.find_column(foreign_key)?;
    let struct_name = model.name;
    let foreign_key_access = foreign_key_field.name.access();
    let foreign_key_ty = &foreign_key_field.ty;
    let table_name = model.table_name();
    let foreign_key_trait = quote!{
        <#foreign_key_ty
            as diesel::associations::ForeignKey<diesel::dsl::SqlTypeOf<#table_name::#foreign_key>>>
    };

    Ok(quote! {
        impl #impl_generics diesel::associations::BelongsTo<#parent_struct>
            for #struct_name #ty_generics
        #where_clause
        {
            type ForeignKey = #foreign_key_trait::KeyType;
            type ForeignKeyColumn = #table_name::#foreign_key;

            fn foreign_key(&self) -> std::option::Option<&Self::ForeignKey> {
                #foreign_key_trait::key(&self#foreign_key_access)
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
            .unwrap_or_else(|| Ok(infer_foreign_key(parent_struct)))?;

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

fn infer_foreign_key(name: syn::Ident) -> syn::Ident {
    let snake_case = camel_to_snake(name.as_ref());
    syn::Ident::new(&format!("{}_id", snake_case), name.span())
}
