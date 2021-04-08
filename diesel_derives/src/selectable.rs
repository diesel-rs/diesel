use crate::diagnostic_shim::Diagnostic;
use model::Model;
use proc_macro2::TokenStream;
use util::wrap_in_dummy_mod;

pub fn derive(item: syn::DeriveInput) -> Result<TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;
    let table_name = &model.table_name();

    let mut expression_ty = Vec::with_capacity(model.fields().len());
    let mut selection = Vec::with_capacity(model.fields().len());

    for field in model.fields() {
        if field.has_flag("embed") {
            let field_ty = field.ty_for_deserialize()?;
            expression_ty.push(quote!(<#field_ty as Selectable>::SelectExpression));
            selection.push(quote!(<#field_ty as Selectable>::selection()));
        } else {
            let column_name = field.column_name();
            expression_ty.push(quote!(#table_name::#column_name));
            selection.push(quote!(#table_name::#column_name));
        }
    }

    let struct_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::query_builder::Selectable;

        impl #impl_generics Selectable for #struct_name #ty_generics
            #where_clause
        {
            type SelectExpression = (#(#expression_ty,)*);

            fn selection() -> Self::SelectExpression {
                (#(#selection,)*)
            }
        }
    }))
}
