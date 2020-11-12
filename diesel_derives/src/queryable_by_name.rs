use proc_macro2::{Span, TokenStream};
use syn::{DeriveInput, Ident, LitStr, Type};

use field::{Field, FieldName};
use model::Model;
use util::wrap_in_dummy_mod;

pub fn derive(item: DeriveInput) -> TokenStream {
    let model = Model::from_item(&item, false);

    let struct_name = &item.ident;
    let fields = &model.fields().iter().map(get_ident).collect::<Vec<_>>();
    let field_names = model.fields().iter().map(|f| &f.name);

    let initial_field_expr = model.fields().iter().map(|f| {
        let field_ty = &f.ty;

        if f.embed {
            quote!(<#field_ty as QueryableByName<__DB>>::build(row)?)
        } else {
            let deserialize_ty = f.ty_for_deserialize();
            let name = f.column_name();
            let name = LitStr::new(&name.to_string(), name.span());
            quote!(
               {
                   let field = diesel::row::NamedRow::get(row, #name)?;
                   <#deserialize_ty as Into<#field_ty>>::into(field)
               }
            )
        }
    });

    let (_, ty_generics, ..) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    for field in model.fields() {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        let field_ty = field.ty_for_deserialize();
        if field.embed {
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

    wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, QueryableByName};
        use diesel::row::{NamedRow};
        use diesel::sql_types::Untyped;

        impl #impl_generics QueryableByName<__DB>
            for #struct_name #ty_generics
        #where_clause
        {
            fn build<'__a>(row: &impl NamedRow<'__a, __DB>) -> deserialize::Result<Self>
            {
                #(
                    let mut #fields = #initial_field_expr;
                )*
                deserialize::Result::Ok(Self {
                    #(
                        #field_names: #fields,
                    )*
                })
            }
        }
    })
}

fn get_ident(field: &Field) -> Ident {
    match &field.name {
        FieldName::Named(n) => n.clone(),
        FieldName::Unnamed(i) => Ident::new(&format!("field_{}", i.index), i.span),
    }
}

fn sql_type(field: &Field, model: &Model) -> Type {
    let table_name = model.table_name();

    match field.sql_type {
        Some(ref st) => st.clone(),
        None => {
            if model.has_table_name_attribute() {
                let column_name = field.column_name();
                parse_quote!(diesel::dsl::SqlTypeOf<#table_name::#column_name>)
            } else {
                let field_name = match field.name {
                    FieldName::Named(ref x) => x.clone(),
                    _ => Ident::new("field", Span::call_site()),
                };
                abort!(
                    field.span, "Cannot determine the SQL type of {}", field_name;
                    help = "Your struct must either be annotated with `#[diesel(table_name = foo)]` or have this field annotated with `#[diesel(sql_type = ...)]`";
                );
            }
        }
    }
}
