use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::DeriveInput;
use syn::Result;

use crate::attrs::AttributeSpanWrapper;
use crate::field::Field;
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
        let Field {
            ty,
            name,
            serialize_as,
            ..
        } = &model.find_column(pk)?;
        if let Some(AttributeSpanWrapper { item: ty, .. }) = serialize_as.as_ref() {
            field_ty.push(quote!(#ty));
            field_name.push(quote!(::std::convert::Into::<#ty>::into(self.#name.clone())));
        } else {
            field_ty.push(quote!(&'ident #ty));
            field_name.push(quote!(&self.#name));
        }
    }

    Ok(wrap_in_dummy_mod(quote! {
        impl #impl_generics diesel::associations::HasTable for #struct_name #ty_generics
        #where_clause
        {
            type Table = #table_name::table;

            fn table() -> <Self as diesel::associations::HasTable>::Table {
                #table_name::table
            }
        }

        impl #ref_generics diesel::associations::Identifiable for &'ident #struct_name #ty_generics
        #where_clause
        {
            type Id = (#(#field_ty),*);

            fn id(self) -> <Self as diesel::associations::Identifiable>::Id {
                (#(#field_name),*)
            }
        }

        impl #ref_generics diesel::associations::Identifiable for &'_ &'ident #struct_name #ty_generics
            #where_clause
        {
            type Id = (#(#field_ty),*);

            fn id(self) -> <Self as diesel::associations::Identifiable>::Id {
                (#(#field_name),*)
            }
        }
    }))
}
