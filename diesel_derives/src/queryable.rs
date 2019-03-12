use proc_macro2;
use syn;

use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = &item.ident;
    let field_ty = model
        .fields()
        .iter()
        .map(|f| f.ty_for_deserialize())
        .collect::<Result<Vec<_>, _>>()?;
    let field_ty = &field_ty;
    let build_expr = model.fields().iter().enumerate().map(|(i, f)| {
        let i = syn::Index::from(i);
        f.name.assign(parse_quote!(row.#i.into()))
    });

    let (_, ty_generics, _) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));
    generics.params.push(parse_quote!(__ST));
    {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!((#(#field_ty,)*): Queryable<__ST, __DB>));
    }
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(wrap_in_dummy_mod(
        model.dummy_mod_name("queryable"),
        quote! {
            use diesel::deserialize::Queryable;

            impl #impl_generics Queryable<__ST, __DB> for #struct_name #ty_generics
            #where_clause
            {
                type Row = <(#(#field_ty,)*) as Queryable<__ST, __DB>>::Row;

                fn build(row: Self::Row) -> Self {
                    let row: (#(#field_ty,)*) = Queryable::build(row);
                    Self {
                        #(#build_expr,)*
                    }
                }
            }
        },
    ))
}
