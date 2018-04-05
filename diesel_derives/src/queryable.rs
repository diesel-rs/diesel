use proc_macro2::Span;
use quote;
use syn;

use field::*;
use meta::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<quote::Tokens, Diagnostic> {
    if let Some(meta) = MetaItem::with_name(&item.attrs, "check_types") {
        if !cfg!(feature = "nightly") {
            meta.span()
                .error(
                    "`#[check_types]` requires the `unstable` feature on Diesel and a nightly compiler",
                )
                .emit();
        }

        return derive_checked(item, meta);
    }
    let model = Model::from_item(&item)?;

    let struct_name = item.ident;
    let field_ty = model.fields().iter().map(|f| &f.ty).collect::<Vec<_>>();
    let field_ty = &field_ty;
    let build_expr = model.fields().iter().enumerate().map(|(i, f)| {
        let i = syn::Index::from(i);
        f.name.assign(parse_quote!(row.#i))
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
            use self::diesel::Queryable;

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

fn derive_checked(
    item: syn::DeriveInput,
    check_types: MetaItem,
) -> Result<quote::Tokens, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = item.ident;
    let backend = check_types.nested_item("backend")?.ty_value()?;
    let sql_tys = model.fields().iter().map(|f| model.sql_type_of(f));
    let field_exprs = model
        .fields()
        .iter()
        .enumerate()
        .map(|(i, f)| field_expr(i, f, &model, &backend));

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(
        model.dummy_mod_name("queryable"),
        quote! {
            use self::diesel::deserialize::{self, Queryable, FromSqlRow};

            pub enum DummyRow {}

            impl<ST> FromSqlRow<ST, #backend> for DummyRow {
                fn build_from_row<R: self::diesel::row::Row<#backend>>(_: &mut R)
                    -> deserialize::Result<Self>
                {
                    use self::std::result::Result::Err;
                    use self::std::convert::Into;

                    Err("`#[check_types]` is only for debugging purposes".into())
                }
            }

            impl #impl_generics Queryable<(#(#sql_tys,)*), #backend>
                for #struct_name #ty_generics
            #where_clause
            {
                type Row = DummyRow;

                #[allow(unreachable_code)]
                fn build(row: Self::Row) -> Self {
                    Self {
                        #(#field_exprs,)*
                    }
                }
            }
        },
    ))
}

fn field_expr(idx: usize, field: &Field, model: &Model, backend: &syn::Type) -> syn::FieldValue {
    let st = model.sql_type_of(field);
    let mut tokens = quote_spanned! {field.span.resolved_at(Span::def_site())=>
        Queryable::<#st, #backend>::build(unimplemented!())
    };
    if field.sql_type.is_none() {
        let table_name = model.table_name();
        let column_name = field.column_name();
        let idx = syn::Index::from(idx);
        tokens = quote_spanned! {column_name.span=>
            {
                let #table_name::#column_name = #table_name::all_columns.#idx;
                #tokens
            }
        }
    }
    field.name.assign(parse_quote!(#tokens))
}
