use proc_macro2::{self, Ident, Span};
use syn;

use field::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = &item.ident;
    let field_expr = model
        .fields()
        .iter()
        .map(|f| field_expr(f, &model))
        .collect::<Result<Vec<_>, _>>()?;

    let (_, ty_generics, ..) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    for field in model.fields() {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        let field_ty = field.ty_for_deserialize()?;
        if field.has_flag("embed") {
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: QueryableByName<__DB>));
        } else {
            let st = sql_type(field, &model);
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: diesel::deserialize::FromSql<#st, __DB>));
        }
    }

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let impl_table_queryable: Option<syn::Item> = if model.has_table_name_attribute() {
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

        let (ty_impl_generics, _, ty_where_caluse) = item.generics.split_for_impl();
        Some(parse_quote!(
            impl #ty_impl_generics TableQueryable
            for #struct_name #ty_generics
            #ty_where_caluse
            {
                type Columns = (#(#field_columns_ty,)*);
                fn columns() -> Self::Columns {
                    (#(#field_columns_inst,)*)
                }
            }
        ))
    } else {
        None
    };

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, QueryableByName, TableQueryable};
        use diesel::row::NamedRow;

        impl #impl_generics QueryableByName<__DB>
            for #struct_name #ty_generics
        #where_clause
        {
            fn build<__R: NamedRow<__DB>>(row: &__R) -> deserialize::Result<Self> {
                std::result::Result::Ok(Self {
                    #(#field_expr,)*
                })
            }
        }

        #impl_table_queryable
    }))
}

fn field_expr(field: &Field, model: &Model) -> Result<syn::FieldValue, Diagnostic> {
    if field.has_flag("embed") {
        Ok(field
            .name
            .assign(parse_quote!(QueryableByName::build(row)?)))
    } else {
        let column_name = field.column_name();
        let ty = field.ty_for_deserialize()?;
        let st = sql_type(field, model);
        Ok(field
            .name
            .assign(parse_quote!(row.get::<#st, #ty>(stringify!(#column_name))?.into())))
    }
}

fn field_column_ty(field: &Field, model: &Model) -> Result<syn::Type, Diagnostic> {
    if field.has_flag("embed") {
        let embed_ty = &field.ty;
        Ok(parse_quote!(<#embed_ty as TableQueryable>::Columns))
    } else {
        let table_name = model.table_name();
        let column_name = field.column_name();
        Ok(parse_quote!(#table_name::#column_name))
    }
}

fn field_column_inst(field: &Field, model: &Model) -> Result<syn::Expr, Diagnostic> {
    if field.has_flag("embed") {
        let embed_ty = &field.ty;
        Ok(parse_quote!(<#embed_ty as TableQueryable>::columns()))
    } else {
        let table_name = model.table_name();
        let column_name = field.column_name();
        Ok(parse_quote!(#table_name::#column_name))
    }
}

fn sql_type(field: &Field, model: &Model) -> syn::Type {
    let table_name = model.table_name();
    let column_name = field.column_name();

    match field.sql_type {
        Some(ref st) => st.clone(),
        None => {
            if model.has_table_name_attribute() {
                parse_quote!(diesel::dsl::SqlTypeOf<#table_name::#column_name>)
            } else {
                let field_name = match field.name {
                    FieldName::Named(ref x) => x.clone(),
                    _ => Ident::new("field", Span::call_site()),
                };
                field
                    .span
                    .error(format!("Cannot determine the SQL type of {}", field_name))
                    .help(
                        "Your struct must either be annotated with `#[table_name = \"foo\"]` \
                         or have all of its fields annotated with `#[sql_type = \"Integer\"]`",
                    )
                    .emit();
                parse_quote!(())
            }
        }
    }
}
