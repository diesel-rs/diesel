use quote;
use syn;

use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<quote::Tokens, Diagnostic> {
    let model = Model::from_item(&item)?;
    let struct_name = model.name;
    let table_name = model.table_name();

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut ref_generics = item.generics.clone();
    ref_generics.params.push(parse_quote!('ident));
    let (ref_generics, ..) = ref_generics.split_for_impl();

    let (field_ty, field_access): (Vec<_>, Vec<_>) = model
        .primary_key_names
        .iter()
        .filter_map(|&pk| model.find_column(pk).emit_error())
        .map(|f| (&f.ty, f.name.access()))
        .unzip();

    Ok(wrap_in_dummy_mod(
        model.dummy_mod_name("identifiable"),
        quote! {
            use self::diesel::associations::{HasTable, Identifiable};

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
                    (#(&self#field_access),*)
                }
            }
        },
    ))
}
