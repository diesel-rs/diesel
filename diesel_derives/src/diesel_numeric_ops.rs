use proc_macro2::{self, Ident, Span};
use syn;

use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let struct_name = &item.ident;

    {
        let where_clause = item
            .generics
            .where_clause
            .get_or_insert(parse_quote!(where));
        where_clause.predicates.push(parse_quote!(Self: Expression));
        where_clause.predicates.push_punct(Default::default());
    }
    let (_, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut impl_generics = item.generics.clone();
    impl_generics.params.push(parse_quote!(__Rhs));
    let (impl_generics, _, _) = impl_generics.split_for_impl();

    let dummy_name = format!("_impl_diesel_numeric_ops_for_{}", item.ident);

    Ok(wrap_in_dummy_mod(
        Ident::new(&dummy_name.to_lowercase(), Span::call_site()),
        quote! {
            use diesel::expression::{ops, Expression, AsExpression};
            use diesel::sql_types::ops::{Add, Sub, Mul, Div};

            impl #impl_generics ::std::ops::Add<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as Expression>::SqlType: Add,
                __Rhs: AsExpression<<<Self as Expression>::SqlType as Add>::Rhs>,
            {
                type Output = ops::Add<Self, __Rhs::Expression>;

                fn add(self, rhs: __Rhs) -> Self::Output {
                    ops::Add::new(self, rhs.as_expression())
                }
            }

            impl #impl_generics ::std::ops::Sub<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as Expression>::SqlType: Sub,
                __Rhs: AsExpression<<<Self as Expression>::SqlType as Sub>::Rhs>,
            {
                type Output = ops::Sub<Self, __Rhs::Expression>;

                fn sub(self, rhs: __Rhs) -> Self::Output {
                    ops::Sub::new(self, rhs.as_expression())
                }
            }

            impl #impl_generics ::std::ops::Mul<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as Expression>::SqlType: Mul,
                __Rhs: AsExpression<<<Self as Expression>::SqlType as Mul>::Rhs>,
            {
                type Output = ops::Mul<Self, __Rhs::Expression>;

                fn mul(self, rhs: __Rhs) -> Self::Output {
                    ops::Mul::new(self, rhs.as_expression())
                }
            }

            impl #impl_generics ::std::ops::Div<__Rhs> for #struct_name #ty_generics
            #where_clause
                <Self as Expression>::SqlType: Div,
                __Rhs: AsExpression<<<Self as Expression>::SqlType as Div>::Rhs>,
            {
                type Output = ops::Div<Self, __Rhs::Expression>;

                fn div(self, rhs: __Rhs) -> Self::Output {
                    ops::Div::new(self, rhs.as_expression())
                }
            }
        },
    ))
}
