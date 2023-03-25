use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::DeriveInput;
use syn::Result;

use crate::model::Model;
use crate::util::wrap_in_dummy_mod;

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, false, false)?;

    let struct_name = &item.ident;
    let table_name = &model.table_names()[0];

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut ref_generics = item.generics.clone();
    ref_generics.params.push(parse_quote!('ident));
    let (ref_generics, ..) = ref_generics.split_for_impl();

    let mut field_ty = Vec::new();
    let mut field_name = Vec::new();
    for pk in model.primary_key_names.iter() {
        let f = model.find_column(pk)?;
        field_ty.push(&f.ty);
        field_name.push(&f.name);
    }

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::associations::{HasTable, Identifiable};

        impl #impl_generics HasTable for #struct_name #ty_generics
        #where_clause
        {
            type Table = #table_name::table;

            fn table() -> Self::Table {
                #table_name::table
            }
        }

        impl #ref_generics Identifiable for &'ident #struct_name #ty_generics
        #where_clause
        {
            type Id = (#(&'ident #field_ty),*);

            fn id(self) -> Self::Id {
                (#(&self.#field_name),*)
            }
        }

        impl #ref_generics Identifiable for &'_ &'ident #struct_name #ty_generics
            #where_clause
        {
            type Id = (#(&'ident #field_ty),*);

            fn id(self) -> Self::Id {
                (#(&self.#field_name),*)
            }
        }
    }))
}
