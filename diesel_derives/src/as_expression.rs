use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Result;
use syn::parse_quote;

use crate::model::Model;
use crate::util::{ty_for_foreign_derive, wrap_in_dummy_mod};

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, true, false)?;
    if model.sql_types.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::mixed_site(),
            "at least one `sql_type` is needed for deriving `AsExpression` on a structure.",
        ));
    }

    let struct_ty = ty_for_foreign_derive(&item, &model)?;

    let sql_types = model.sql_types;

    let tokens = derive_inner(
        sql_types,
        item.generics.clone(),
        struct_ty,
        model.foreign_derive,
        model.not_sized,
    )?;

    Ok(wrap_in_dummy_mod(tokens))
}

pub fn derive_inner(
    sql_types: Vec<syn::Type>,
    generics: syn::Generics,
    struct_ty: syn::Type,
    foreign_derive: bool,
    not_sized: bool,
) -> Result<TokenStream> {
    // type generics are already handled by `ty_for_foreign_derive`
    let (impl_generics_plain, _, where_clause_plain) = generics.split_for_impl();

    let mut generics1 = generics.clone();
    generics1.params.push(parse_quote!('__expr));
    let (impl_generics, _, where_clause) = generics1.split_for_impl();

    let mut generics2 = generics1.clone();
    generics2.params.push(parse_quote!('__expr2));
    let (impl_generics2, _, where_clause2) = generics2.split_for_impl();

    // Smart-pointer wrapper impls (`Rc<T>`, `Arc<T>`, `Box<T>`) are emitted only for
    // `#[diesel(foreign_derive)]` types. For user-defined local types they would
    // run into both coherence (E0119) and orphan (E0117) checks against the wildcard
    // `impl<T, ST> AsExpression<ST> for T where T: Expression<SqlType=ST>` once
    // `Expression for Rc/Arc/Box<T>` exists, because Rust treats `MyLocalType:
    // Expression` as downstream-extensible. Diesel's own `foreign_impls` cover the
    // standard-library primitives (`String`, `i32`, `bool`, ...) so users get
    // `Rc<String>`/`Arc<String>` field support automatically.
    let wrappers: Vec<syn::Path> = if foreign_derive {
        vec![
            parse_quote!(alloc::rc::Rc),
            parse_quote!(alloc::sync::Arc),
            parse_quote!(alloc::boxed::Box),
        ]
    } else {
        Vec::new()
    };

    let tokens = sql_types.iter().map(|sql_type| {

        let mut to_sql_generics = generics.clone();
        to_sql_generics.params.push(parse_quote!(__DB));
        to_sql_generics.make_where_clause().predicates.push(parse_quote!(__DB: diesel::backend::Backend));
        to_sql_generics.make_where_clause().predicates.push(parse_quote!(Self: diesel::serialize::ToSql<#sql_type, __DB>));
        let (to_sql_impl_generics, _, to_sql_where_clause) = to_sql_generics.split_for_impl();

        let wrapper_borrowed = wrappers.iter().map(|wrapper| quote!(
            #[diagnostic::do_not_recommend]
            impl #impl_generics diesel::expression::AsExpression<#sql_type>
                for &'__expr #wrapper<#struct_ty> #where_clause
            {
                type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, &'__expr #struct_ty>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(&**self)
                }
            }

            #[diagnostic::do_not_recommend]
            impl #impl_generics diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>
                for &'__expr #wrapper<#struct_ty> #where_clause
            {
                type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, &'__expr #struct_ty>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(&**self)
                }
            }
        ));
        let wrapper_borrowed = quote!( #( #wrapper_borrowed )* );

        let tokens = quote!(
            impl #impl_generics diesel::expression::AsExpression<#sql_type>
                for &'__expr #struct_ty #where_clause
            {
                type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            #[diagnostic::do_not_recommend]
            impl #impl_generics diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>
                for &'__expr #struct_ty #where_clause
            {
                type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            #[diagnostic::do_not_recommend]
            impl #impl_generics2 diesel::expression::AsExpression<#sql_type>
                for &'__expr2 &'__expr #struct_ty #where_clause2
            {
                type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            #[diagnostic::do_not_recommend]
            impl #impl_generics2 diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>
                for &'__expr2 &'__expr #struct_ty #where_clause2
            {
                type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            impl #to_sql_impl_generics diesel::serialize::ToSql<diesel::sql_types::Nullable<#sql_type>, __DB>
                for #struct_ty #to_sql_where_clause
            {
                fn to_sql<'__b>(&'__b self, out: &mut diesel::serialize::Output<'__b, '_, __DB>) -> diesel::serialize::Result
                {
                    diesel::serialize::ToSql::<#sql_type, __DB>::to_sql(self, out)
                }
            }

            #wrapper_borrowed
        );

        // Owned smart-pointer wrapper impls. Emitted regardless of `model.not_sized`
        // because `Rc<T>`, `Arc<T>`, and `Box<T>` are always `Sized` even when `T` is
        // `?Sized` (e.g., `Rc<str>`, `Box<[u8]>`).
        let wrapper_owned = wrappers.iter().map(|wrapper| quote!(
            impl #impl_generics_plain diesel::expression::AsExpression<#sql_type>
                for #wrapper<#struct_ty> #where_clause_plain
            {
                type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            #[diagnostic::do_not_recommend]
            impl #impl_generics_plain diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>
                for #wrapper<#struct_ty> #where_clause_plain
            {
                type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }
        ));
        let wrapper_owned = quote!( #( #wrapper_owned )* );

        if not_sized {
            quote!(
                #tokens

                #wrapper_owned
            )
        } else {
            quote!(
                #tokens

                impl #impl_generics_plain diesel::expression::AsExpression<#sql_type> for #struct_ty #where_clause_plain {
                    type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, Self>;

                    fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                        diesel::internal::derives::as_expression::Bound::new(self)
                    }
                }

                impl #impl_generics_plain diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>> for #struct_ty
                #where_clause_plain
                {
                    type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, Self>;

                    fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                        diesel::internal::derives::as_expression::Bound::new(self)
                    }
                }

                #wrapper_owned
            )
        }
    });
    Ok(quote! {#(#tokens)*})
}
