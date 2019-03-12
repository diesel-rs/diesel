use proc_macro2::*;
use quote::ToTokens;
use syn::parse::{self, Parse, ParseStream};
use syn::punctuated::Punctuated;

use meta::*;
use util::*;

// Extremely curious why this triggers on a nearly branchless function
#[allow(clippy::cyclomatic_complexity)]
pub(crate) fn expand(input: SqlFunctionDecl) -> Result<TokenStream, Diagnostic> {
    let SqlFunctionDecl {
        mut attributes,
        fn_token,
        fn_name,
        mut generics,
        args,
        return_type,
    } = input;

    let sql_name = MetaItem::with_name(&attributes, "sql_name")
        .map(|m| m.str_value())
        .unwrap_or_else(|| Ok(fn_name.to_string()))?;
    let is_aggregate = MetaItem::with_name(&attributes, "aggregate").is_some();

    attributes.retain(|attr| {
        attr.interpret_meta()
            .map(|m| m.name() != "sql_name" && m.name() != "aggregate")
            .unwrap_or(true)
    });

    let args = &args;
    let (ref arg_name, ref arg_type): (Vec<_>, Vec<_>) = args
        .iter()
        .map(|StrictFnArg { name, ty, .. }| (name, ty))
        .unzip();
    let arg_struct_assign = args.iter().map(
        |StrictFnArg {
             name, colon_token, ..
         }| {
            let name2 = name.clone();
            quote!(#name #colon_token #name2.as_expression())
        },
    );

    let type_args = &generics
        .type_params()
        .map(|type_param| type_param.ident.clone())
        .collect::<Vec<_>>();
    let type_args2 = &type_args.clone();
    for StrictFnArg { name, .. } in args {
        generics.params.push(parse_quote!(#name));
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // Even if we force an empty where clause, it still won't print the where
    // token with no bounds.
    let where_clause = where_clause
        .map(|w| quote!(#w))
        .unwrap_or_else(|| quote!(where));

    let mut generics_with_internal = generics.clone();
    generics_with_internal
        .params
        .push(parse_quote!(__DieselInternal));
    let (impl_generics_internal, _, _) = generics_with_internal.split_for_impl();

    let mut tokens = quote! {
        use diesel::{self, QueryResult};
        use diesel::expression::{AsExpression, Expression, SelectableExpression, AppearsOnTable};
        use diesel::query_builder::{QueryFragment, AstPass};
        use diesel::sql_types::*;
        use super::*;

        #[derive(Debug, Clone, Copy, QueryId, DieselNumericOps)]
        pub struct #fn_name #ty_generics {
            #(pub(in super) #args,)*
            #(pub(in super) #type_args: ::std::marker::PhantomData<#type_args2>,)*
        }

        pub type HelperType #ty_generics = #fn_name <
            #(#type_args,)*
            #(<#arg_name as AsExpression<#arg_type>>::Expression,)*
        >;

        impl #impl_generics Expression for #fn_name #ty_generics
        #where_clause
            (#(#arg_name),*): Expression,
        {
            type SqlType = #return_type;
        }

        // __DieselInternal is what we call QS normally
        impl #impl_generics_internal SelectableExpression<__DieselInternal>
            for #fn_name #ty_generics
        #where_clause
            #(#arg_name: SelectableExpression<__DieselInternal>,)*
            Self: AppearsOnTable<__DieselInternal>,
        {
        }

        // __DieselInternal is what we call QS normally
        impl #impl_generics_internal AppearsOnTable<__DieselInternal>
            for #fn_name #ty_generics
        #where_clause
            #(#arg_name: AppearsOnTable<__DieselInternal>,)*
            Self: Expression,
        {
        }

        // __DieselInternal is what we call DB normally
        impl #impl_generics_internal QueryFragment<__DieselInternal>
            for #fn_name #ty_generics
        where
            __DieselInternal: diesel::backend::Backend,
            for<'a> (#(&'a #arg_name),*): QueryFragment<__DieselInternal>,
        {
            fn walk_ast(&self, mut out: AstPass<__DieselInternal>) -> QueryResult<()> {
                out.push_sql(concat!(#sql_name, "("));
                (#(&self.#arg_name,)*).walk_ast(out.reborrow())?;
                out.push_sql(")");
                Ok(())
            }
        }
    };

    if !is_aggregate {
        tokens = quote! {
            #tokens

            impl #impl_generics diesel::expression::NonAggregate
                for #fn_name #ty_generics
            #where_clause
                #(#arg_name: diesel::expression::NonAggregate,)*
            {
            }
        };

        if cfg!(feature = "sqlite") && type_args.is_empty() {
            tokens = quote! {
                #tokens

                use diesel::sqlite::{Sqlite, SqliteConnection};
                use diesel::serialize::ToSql;
                use diesel::deserialize::Queryable;

                #[allow(dead_code)]
                /// Registers an implementation for this function on the given connection
                ///
                /// This function must be called for every `SqliteConnection` before
                /// this SQL function can be used on SQLite. The implementation must be
                /// deterministic (returns the same result given the same arguments). If
                /// the function is nondeterministic, call
                /// `register_nondeterministic_impl` instead.
                pub fn register_impl<F, Ret, #(#arg_name,)*>(
                    conn: &SqliteConnection,
                    f: F,
                ) -> QueryResult<()>
                where
                    F: Fn(#(#arg_name,)*) -> Ret + Send + 'static,
                    (#(#arg_name,)*): Queryable<(#(#arg_type,)*), Sqlite>,
                    Ret: ToSql<#return_type, Sqlite>,
                {
                    conn.register_sql_function::<(#(#arg_type,)*), #return_type, _, _, _>(
                        #sql_name,
                        true,
                        move |(#(#arg_name,)*)| f(#(#arg_name,)*),
                    )
                }

                #[allow(dead_code)]
                /// Registers an implementation for this function on the given connection
                ///
                /// This function must be called for every `SqliteConnection` before
                /// this SQL function can be used on SQLite.
                /// `register_nondeterministic_impl` should only be used if your
                /// function can return different results with the same arguments (e.g.
                /// `random`). If your function is deterministic, you should call
                /// `register_impl` instead.
                pub fn register_nondeterministic_impl<F, Ret, #(#arg_name,)*>(
                    conn: &SqliteConnection,
                    mut f: F,
                ) -> QueryResult<()>
                where
                    F: FnMut(#(#arg_name,)*) -> Ret + Send + 'static,
                    (#(#arg_name,)*): Queryable<(#(#arg_type,)*), Sqlite>,
                    Ret: ToSql<#return_type, Sqlite>,
                {
                    conn.register_sql_function::<(#(#arg_type,)*), #return_type, _, _, _>(
                        #sql_name,
                        false,
                        move |(#(#arg_name,)*)| f(#(#arg_name,)*),
                    )
                }
            };
        }
    }

    tokens = quote! {
        #(#attributes)*
        #[allow(non_camel_case_types)]
        pub #fn_token #fn_name #impl_generics (#(#args,)*)
            -> #fn_name::HelperType #ty_generics
        #where_clause
            #(#arg_name: ::diesel::expression::AsExpression<#arg_type>,)*
        {
            #fn_name::#fn_name {
                #(#arg_struct_assign,)*
                #(#type_args: ::std::marker::PhantomData,)*
            }
        }

        #[doc(hidden)]
        #[allow(non_camel_case_types, non_snake_case, unused_imports)]
        pub(crate) mod #fn_name {
            #tokens
        }
    };

    Ok(tokens)
}

pub(crate) struct SqlFunctionDecl {
    attributes: Vec<syn::Attribute>,
    fn_token: Token![fn],
    fn_name: syn::Ident,
    generics: syn::Generics,
    args: Punctuated<StrictFnArg, Token![,]>,
    return_type: syn::Type,
}

impl Parse for SqlFunctionDecl {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let attributes = syn::Attribute::parse_outer(input)?;
        let fn_token: Token![fn] = input.parse()?;
        let fn_name = syn::Ident::parse(input)?;
        let generics = syn::Generics::parse(input)?;
        let args;
        let _paren = parenthesized!(args in input);
        let args = args.parse_terminated::<_, Token![,]>(StrictFnArg::parse)?;
        let return_type = if Option::<Token![->]>::parse(input)?.is_some() {
            syn::Type::parse(input)?
        } else {
            parse_quote!(())
        };
        let _semi = Option::<Token![;]>::parse(input)?;

        Ok(Self {
            attributes,
            fn_token,
            fn_name,
            generics,
            args,
            return_type,
        })
    }
}

/// Essentially the same as syn::ArgCaptured, but only allowing ident patterns
struct StrictFnArg {
    name: syn::Ident,
    colon_token: Token![:],
    ty: syn::Type,
}

impl Parse for StrictFnArg {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let name = input.parse()?;
        let colon_token = input.parse()?;
        let ty = input.parse()?;
        Ok(Self {
            name,
            colon_token,
            ty,
        })
    }
}

impl ToTokens for StrictFnArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.colon_token.to_tokens(tokens);
        self.name.to_tokens(tokens);
    }
}
