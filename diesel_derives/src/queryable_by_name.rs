use quote::Tokens;
use syn;

use model::Model;
use util::wrap_item_in_const;

pub fn derive(item: syn::DeriveInput) -> Tokens {
    let model = t!(Model::from_item(&item, "QueryableByName"));

    let generics = syn::aster::from_generics(model.generics.clone())
        .ty_param_id("__DB")
        .build();
    let struct_ty = &model.ty;
    let table_name = model.table_name();

    let attr_where_clause = model.attrs.iter().map(|attr| {
        let attr_ty = &attr.ty;
        let column_name = attr.column_name();
        quote! {
            __DB: diesel::types::HasSqlType<diesel::dsl::SqlTypeOf<#table_name::#column_name>>,
            #attr_ty: diesel::types::FromSql<diesel::dsl::SqlTypeOf<#table_name::#column_name>, __DB>,
        }
    });

    let build_expr = build_expr_for_model(&model);

    let model_name_uppercase = model.name.as_ref().to_uppercase();
    let dummy_const = format!("_IMPL_QUERYABLE_BY_NAME_FOR_{}", model_name_uppercase).into();

    wrap_item_in_const(
        dummy_const,
        quote!(
            impl#generics diesel::query_source::QueryableByName<__DB> for #struct_ty where
                __DB: diesel::backend::Backend,
                #(#attr_where_clause)*
            {
               fn build<__R: diesel::row::NamedRow<__DB>>(row: &__R) -> Result<Self, Box<::std::error::Error + Send + Sync>> {
                   Ok(#build_expr)
               }
            }
        ),
    )
}

fn build_expr_for_model(model: &Model) -> Tokens {
    let table_name = model.table_name();
    let attr_exprs = model.attrs.iter().map(|attr| {
        let name = attr.field_name();
        let column_name = attr.column_name();
        let st = quote!(diesel::dsl::SqlTypeOf<#table_name::#column_name>);
        quote!(#name: diesel::row::NamedRow::get::<#st, _>(row, stringify!(#column_name))?)
    });

    quote!(Self {
        #(#attr_exprs,)*
    })
}
