use quote::Tokens;
use syn;

use model::Model;

pub fn derive_queryable(item: syn::MacroInput) -> Tokens {
    let model = t!(Model::from_item(&item, "Queryable"));
    
    let struct_name = &model.name;
    let ty_params = &model.generics.ty_params;
    let lifetimes = &model.generics.lifetimes;
    let struct_ty = if !ty_params.is_empty() || !lifetimes.is_empty(){
        quote!(#struct_name<#(#lifetimes,)* #(#ty_params,)*>)
    } else {
        quote!(#struct_name)
    };
    let row_ty = model.attrs.iter().map(|a| &a.ty);
    let field_names = model.attrs.iter().enumerate().map(|(counter, a)| a.field_name.clone()
        .unwrap_or_else(||{
            syn::Ident::from(format!("t_{}", counter))
    }));
    
    let row_ty = quote!((#(#row_ty,)*));
    let row_pat = quote!((#(#field_names,)*));
    let field_names = model.attrs.iter().enumerate().map(|(counter, a)|
         a.field_name.clone().map(|name| {
             quote!(#name:#name)
         })
        .unwrap_or_else(||{
            let r = syn::Ident::from(format!("t_{}", counter));
            quote!(#r)
    }));
    let build_expr = if model.attrs[0].field_name.is_some(){
        quote!(#struct_name {#(#field_names,)*})
    } else {
        quote!(#struct_name (#(#field_names,)*))
    };

    let dummy_const = syn::Ident::new(format!("_IMPL_QUERYABLE_FOR_{}", struct_name));

    quote!(
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate diesel as _diesel;
            #[automatically_derived]
            impl<#(#lifetimes,)* #(#ty_params,)* __DB, __ST> _diesel::Queryable<__ST, __DB> for #struct_ty where
                __DB: _diesel::backend::Backend + _diesel::types::HasSqlType<__ST>,
                #row_ty: _diesel::types::FromSqlRow<__ST, __DB>,
            {
               type Row = #row_ty;
               
               fn build(row: Self::Row) -> Self {
                   let #row_pat = row;
                   #build_expr
               }
            }
        };)
}

