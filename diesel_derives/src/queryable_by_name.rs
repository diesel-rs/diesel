use quote::Tokens;
use syn;

use attr::Attr;
use model::Model;
use util::wrap_item_in_const;

pub fn derive(item: syn::DeriveInput) -> Tokens {
    let model = t!(Model::from_item(&item, "QueryableByName"));

    let generics = syn::aster::from_generics(model.generics.clone())
        .ty_param_id("__DB")
        .build();
    let struct_ty = &model.ty;

    let attr_where_clause = model.attrs.iter().map(|attr| {
        let attr_ty = &attr.ty;
        if attr.has_flag("embed") {
            quote!(#attr_ty: diesel::deserialize::QueryableByName<__DB>,)
        } else {
            let st = sql_type(attr, &model);
            quote!(
                #attr_ty: diesel::deserialize::FromSql<#st, __DB>,
            )
        }
    });

    let build_expr = build_expr_for_model(&model);

    wrap_item_in_const(
        model.dummy_const_name("QUERYABLE_BY_NAME"),
        quote!(
            impl#generics diesel::deserialize::QueryableByName<__DB> for #struct_ty where
                __DB: diesel::backend::Backend,
                #(#attr_where_clause)*
            {
               fn build<__R: diesel::row::NamedRow<__DB>>(row: &__R) -> diesel::deserialize::Result<Self> {
                   Ok(#build_expr)
               }
            }
        ),
    )
}

fn build_expr_for_model(model: &Model) -> Tokens {
    let attr_exprs = model.attrs.iter().map(|attr| {
        let name = attr.field_name();
        if attr.has_flag("embed") {
            quote!(#name: diesel::deserialize::QueryableByName::build(row)?)
        } else {
            let column_name = attr.column_name();
            let st = sql_type(attr, model);
            quote!(#name: diesel::row::NamedRow::get::<#st, _>(row, stringify!(#column_name))?)
        }
    });

    quote!(Self {
        #(#attr_exprs,)*
    })
}

fn sql_type(attr: &Attr, model: &Model) -> Tokens {
    let table_name = model.table_name();
    let column_name = attr.column_name();

    match attr.sql_type() {
        Some(st) => quote!(#st),
        None => if model.has_table_name_annotation() {
            quote!(diesel::dsl::SqlTypeOf<#table_name::#column_name>)
        } else {
            quote!(compile_error!(
                "Your struct must either be annotated with `#[table_name = \"foo\"]` or have all of its fields annotated with `#[sql_type = \"Integer\"]`"
            ))
        },
    }
}
