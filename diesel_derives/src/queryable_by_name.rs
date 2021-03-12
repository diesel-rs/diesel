use diagnostic_shim::*;
use proc_macro2::{self, Ident, Span};
use syn;
use syn::spanned::Spanned;

use field::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = &item.ident;
    let fields = model.fields().iter().map(get_ident).collect::<Vec<_>>();
    let field_names = model.fields().iter().map(|f| &f.name).collect::<Vec<_>>();

    let initial_field_expr = model
        .fields()
        .iter()
        .map(|f| {
            if f.has_flag("embed") {
                let field_ty = &f.ty;
                Ok(quote!(<#field_ty as QueryableByName<__DB>>::build(
                    row,
                )?))
            } else {
                let name = f.column_name();
                let field_ty = &f.ty;
                let deserialize_ty = f.ty_for_deserialize()?;
                Ok(quote!(
                   {
                       let field = diesel::row::NamedRow::get(row, stringify!(#name))?;
                       <#deserialize_ty as Into<#field_ty>>::into(field)
                   }
                ))
            }
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;

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
            check_sql_type(&field_ty, &st);
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: diesel::deserialize::FromSql<#st, __DB>));
        }
    }

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
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
    }))
}

fn get_ident(field: &Field) -> Ident {
    match &field.name {
        FieldName::Named(n) => n.clone(),
        FieldName::Unnamed(i) => Ident::new(&format!("field_{}", i.index), Span::call_site()),
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

fn check_sql_type(field_ty: &syn::Type, st: &syn::Type) {
    // syn heapsize example trick won't work here because there is an additional database generic
    // unless we create another trait without the DB part
    //
    //     quote_spanned! {field_ty.span()=>
    //         function(st)
    //     }
    //
    // So easier to just manually check as they will always be the same.
    let ident = |s| Ident::new(s, Span::call_site());
    if let syn::Type::Path(st_path) = st {
        let sql_type_name = st_path
            .path
            .segments
            .last()
            .expect("should have at least one segment");
        if sql_type_name.ident == ident("BigInt") {
            match field_ty {
                syn::Type::Path(ty_path) if ty_path.path.is_ident(&ident("i64")) => {}
                syn::Type::Path(ty_path) => DiagnosticExt::span_help(
                    ty_path.span().error(format!(
                        "{} is not implemented for {}",
                        ty_path.path.get_ident().unwrap(),
                        ident("BigInt").to_string()
                    )),
                    st_path.span(),
                    format!("{} implements {:?}", ident("BigInt").to_string(), "i64"),
                )
                .emit(),
                _ => {} // not sure if this path is possible
            }
        }
    }
}
