use field::*;
use model::*;
use proc_macro2;
use syn;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = item.ident;
    let field_expr = model.fields().iter().map(|f| field_expr(f, &model));

    let (_, ty_generics, ..) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    for field in model.fields() {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        let field_ty = &field.ty;
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

    Ok(wrap_in_dummy_mod(
        model.dummy_mod_name("queryable_by_name"),
        quote! {
            use self::diesel::deserialize::{self, QueryableByName};
            use self::diesel::row::NamedRow;

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
        },
    ))
}

fn field_expr(field: &Field, model: &Model) -> syn::FieldValue {
    if field.has_flag("embed") {
        field
            .name
            .assign(parse_quote!(QueryableByName::build(row)?))
    } else {
        let column_name = field.column_name();
        let st = sql_type(field, model);
        field
            .name
            .assign(parse_quote!(row.get::<#st, _>(stringify!(#column_name))?))
    }
}

fn sql_type(field: &Field, model: &Model) -> syn::Type {
    let table_name = model.table_name();
    let column_name = field.column_name();

    match field.sql_type {
        Some(ref st) => st.clone(),
        None => if model.has_table_name_attribute() {
            parse_quote!(diesel::dsl::SqlTypeOf<#table_name::#column_name>)
        } else {
            let field_name = match field.name {
                FieldName::Named(ref x) => x.to_string(),
                _ => "field".into(),
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
        },
    }
}
