use proc_macro2::{self, Ident, Span};
use syn;

use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let struct_name = &item.ident;

    {
        let where_clause = item.generics
            .where_clause
            .get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!(Self: diesel::expression::Expression));
        where_clause.predicates.push_punct(Default::default());
    }
    let (_, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut impl_generics = item.generics.clone();
    impl_generics.params.push(parse_quote!(__Rhs));
    let (impl_generics, _, _) = impl_generics.split_for_impl();

    let dummy_name = format!("_impl_diesel_numeric_ops_for_{}", item.ident);

    Ok(wrap_in_dummy_mod(
        Ident::new(&dummy_name.to_uppercase(), Span::call_site()),
        quote! {

            impl #impl_generics ::std::ops::Add<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as diesel::expression::Expression>::SqlType: diesel::sql_types::ops::Add,
                  __Rhs: diesel::expression::AsExpression<
                    <<Self as diesel::expression::Expression>::SqlType
                        as diesel::sql_types::ops::Add>::Rhs
                >,
            {
                type Output = diesel::expression::ops::Add<Self, __Rhs::Expression>;

                fn add(self, rhs: __Rhs) -> Self::Output {
                    diesel::expression::ops::Add::new(self, rhs.as_expression())
                }
            }

            impl #impl_generics ::std::ops::Sub<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as diesel::expression::Expression>::SqlType: diesel::sql_types::ops::Sub,
                __Rhs: diesel::expression::AsExpression<
                    <<Self as diesel::expression::Expression>::SqlType
                        as diesel::sql_types::ops::Sub>::Rhs
                >,
            {
                type Output = diesel::expression::ops::Sub<Self, __Rhs::Expression>;

                fn sub(self, rhs: __Rhs) -> Self::Output {
                    diesel::expression::ops::Sub::new(self, rhs.as_expression())
                }
            }

            impl #impl_generics ::std::ops::Mul<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as diesel::expression::Expression>::SqlType: diesel::sql_types::ops::Mul,
                __Rhs: diesel::expression::AsExpression<
                    <<Self as diesel::expression::Expression>::SqlType
                        as diesel::sql_types::ops::Mul>::Rhs
                >,
            {
                type Output = diesel::expression::ops::Mul<Self, __Rhs::Expression>;

                fn mul(self, rhs: __Rhs) -> Self::Output {
                    diesel::expression::ops::Mul::new(self, rhs.as_expression())
                }
            }

            impl #impl_generics ::std::ops::Div<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as diesel::expression::Expression>::SqlType: diesel::sql_types::ops::Div,
                __Rhs: diesel::expression::AsExpression<
                    <<Self as diesel::expression::Expression>::SqlType
                        as diesel::sql_types::ops::Div>::Rhs
                >,
            {
                type Output = diesel::expression::ops::Div<Self, __Rhs::Expression>;

                fn div(self, rhs: __Rhs) -> Self::Output {
                    diesel::expression::ops::Div::new(self, rhs.as_expression())
                }
            }
        },
    ))
}
