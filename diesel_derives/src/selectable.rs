use proc_macro2;
use syn;

use field::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let struct_name = &item.ident;

    let field_columns_ty = model
        .fields()
        .iter()
        .map(|f| field_column_ty(f, &model))
        .collect::<Result<Vec<_>, _>>()?;
    let field_columns_inst = model
        .fields()
        .iter()
        .map(|f| field_column_inst(f, &model))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::expression::Selectable;

        impl #impl_generics Selectable
            for #struct_name #ty_generics
        #where_clause
        {
            type Expression = (#(#field_columns_ty,)*);
            fn new_expression() -> Self::Expression {
                (#(#field_columns_inst,)*)
            }
        }
    }))
}

fn field_column_ty(field: &Field, model: &Model) -> Result<syn::Type, Diagnostic> {
    if field.has_flag("embed") {
        let embed_ty = &field.ty;
        Ok(parse_quote!(<#embed_ty as Selectable>::Expression))
    } else {
        let table_name = model.table_name();
        let column_name = field.column_name();
        Ok(parse_quote!(#table_name::#column_name))
    }
}

fn field_column_inst(field: &Field, model: &Model) -> Result<syn::Expr, Diagnostic> {
    if field.has_flag("embed") {
        let embed_ty = &field.ty;
        Ok(parse_quote!(<#embed_ty as Selectable>::new_expression()))
    } else {
        let table_name = model.table_name();
        let column_name = field.column_name();
        Ok(parse_quote!(#table_name::#column_name))
    }
}
