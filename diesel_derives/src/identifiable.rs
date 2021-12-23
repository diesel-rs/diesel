use proc_macro2::TokenStream;
use syn::DeriveInput;

use model::Model;
use util::wrap_in_dummy_mod;

pub fn derive(item: DeriveInput) -> TokenStream {
    let model = Model::from_item(&item, false);

    let struct_name = &item.ident;
    let table_name = model.table_name();

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut ref_generics = item.generics.clone();
    ref_generics.params.push(parse_quote!('ident));
    let (ref_generics, ..) = ref_generics.split_for_impl();

    let (field_ty, field_name): (Vec<_>, Vec<_>) = model
        .primary_key_names
        .iter()
        .map(|pk| model.find_column(pk))
        .map(|f| (&f.ty, &f.name))
        .unzip();

    wrap_in_dummy_mod(quote! {
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
    })
}
