use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, GenericArgument, Generics, Ident, Lit, Meta, MetaNameValue, PathArguments, Type,
};

pub(crate) fn expand(input: SqlFunctionDecl) -> TokenStream {
    let SqlFunctionDecl {
        mut attributes,
        fn_token,
        fn_name,
        mut generics,
        args,
        return_type,
    } = input;

    let sql_name = attributes
        .iter()
        .find(|attr| {
            attr.parse_meta()
                .map(|m| m.path().is_ident("sql_name"))
                .unwrap_or(false)
        })
        .and_then(|attr| {
            if let Ok(Meta::NameValue(MetaNameValue {
                lit: Lit::Str(lit), ..
            })) = attr.parse_meta()
            {
                Some(lit.value())
            } else {
                None
            }
        })
        .unwrap_or_else(|| fn_name.to_string());

    let is_aggregate = attributes.iter().any(|attr| {
        attr.parse_meta()
            .map(|m| m.path().is_ident("aggregate"))
            .unwrap_or(false)
    });

    attributes.retain(|attr| {
        attr.parse_meta()
            .map(|m| !m.path().is_ident("sql_name") && !m.path().is_ident("aggregate"))
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

    let sql_type;
    let numeric_derive;

    if arg_name.is_empty() {
        sql_type = None;
        // FIXME: We can always derive once trivial bounds are stable
        numeric_derive = None;
    } else {
        sql_type = Some(quote!((#(#arg_name),*): Expression,));
        numeric_derive = Some(quote!(#[derive(diesel::sql_types::DieselNumericOps)]));
    }

    let args_iter = args.iter();
    let mut tokens = quote! {
        use diesel::{self, QueryResult};
        use diesel::expression::{AsExpression, Expression, SelectableExpression, AppearsOnTable, ValidGrouping};
        use diesel::query_builder::{QueryFragment, AstPass};
        use diesel::sql_types::*;
        use super::*;

        #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId)]
        #numeric_derive
        pub struct #fn_name #ty_generics {
            #(pub(in super) #args_iter,)*
            #(pub(in super) #type_args: ::std::marker::PhantomData<#type_args>,)*
        }

        pub type HelperType #ty_generics = #fn_name <
            #(#type_args,)*
            #(<#arg_name as AsExpression<#arg_type>>::Expression,)*
        >;

        impl #impl_generics Expression for #fn_name #ty_generics
        #where_clause
            #sql_type
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

    let is_supported_on_sqlite = cfg!(feature = "sqlite")
        && type_args.is_empty()
        && is_sqlite_type(&return_type)
        && arg_type.iter().all(|a| is_sqlite_type(a));

    if is_aggregate {
        tokens = quote! {
            #tokens

            impl #impl_generics_internal ValidGrouping<__DieselInternal>
                for #fn_name #ty_generics
            {
                type IsAggregate = diesel::expression::is_aggregate::Yes;
            }
        };
        if is_supported_on_sqlite {
            tokens = quote! {
                #tokens

                use diesel::sqlite::{Sqlite, SqliteConnection};
                use diesel::serialize::ToSql;
                use diesel::deserialize::{FromSqlRow, StaticallySizedRow};
                use diesel::sqlite::SqliteAggregateFunction;
                use diesel::sql_types::IntoNullable;
            };

            match arg_name.len() {
                x if x > 1 => {
                    tokens = quote! {
                        #tokens

                        #[allow(dead_code)]
                        /// Registers an implementation for this aggregate function on the given connection
                        ///
                        /// This function must be called for every `SqliteConnection` before
                        /// this SQL function can be used on SQLite. The implementation must be
                        /// deterministic (returns the same result given the same arguments).
                        pub fn register_impl<A, #(#arg_name,)*>(
                            conn: &mut SqliteConnection
                        ) -> QueryResult<()>
                            where
                            A: SqliteAggregateFunction<(#(#arg_name,)*)>
                                + Send
                                + 'static
                                + ::std::panic::UnwindSafe
                                + ::std::panic::RefUnwindSafe,
                            A::Output: ToSql<#return_type, Sqlite>,
                            (#(#arg_name,)*): FromSqlRow<(#(#arg_type,)*), Sqlite> +
                                StaticallySizedRow<(#(#arg_type,)*), Sqlite> +
                                ::std::panic::UnwindSafe,
                        {
                            conn.register_aggregate_function::<(#(#arg_type,)*), #return_type, _, _, A>(#sql_name)
                        }
                    };
                }
                x if x == 1 => {
                    let arg_name = arg_name[0];
                    let arg_type = arg_type[0];

                    tokens = quote! {
                        #tokens

                        #[allow(dead_code)]
                        /// Registers an implementation for this aggregate function on the given connection
                        ///
                        /// This function must be called for every `SqliteConnection` before
                        /// this SQL function can be used on SQLite. The implementation must be
                        /// deterministic (returns the same result given the same arguments).
                        pub fn register_impl<A, #arg_name>(
                            conn: &mut SqliteConnection
                        ) -> QueryResult<()>
                            where
                            A: SqliteAggregateFunction<#arg_name>
                                + Send
                                + 'static
                                + std::panic::UnwindSafe
                                + std::panic::RefUnwindSafe,
                            A::Output: ToSql<#return_type, Sqlite>,
                            #arg_name: FromSqlRow<#arg_type, Sqlite> +
                                StaticallySizedRow<#arg_type, Sqlite> +
                                ::std::panic::UnwindSafe,
                            {
                                conn.register_aggregate_function::<#arg_type, #return_type, _, _, A>(#sql_name)
                            }
                    };
                }
                _ => (),
            }
        }
    } else {
        tokens = quote! {
            #tokens

            #[derive(ValidGrouping)]
            pub struct __Derived<#(#arg_name,)*>(#(#arg_name,)*);

            impl #impl_generics_internal ValidGrouping<__DieselInternal>
                for #fn_name #ty_generics
            where
                __Derived<#(#arg_name,)*>: ValidGrouping<__DieselInternal>,
            {
                type IsAggregate = <__Derived<#(#arg_name,)*> as ValidGrouping<__DieselInternal>>::IsAggregate;
            }
        };

        if is_supported_on_sqlite && !arg_name.is_empty() {
            tokens = quote! {
                #tokens

                use diesel::sqlite::{Sqlite, SqliteConnection};
                use diesel::serialize::ToSql;
                use diesel::deserialize::{FromSqlRow, StaticallySizedRow};

                #[allow(dead_code)]
                /// Registers an implementation for this function on the given connection
                ///
                /// This function must be called for every `SqliteConnection` before
                /// this SQL function can be used on SQLite. The implementation must be
                /// deterministic (returns the same result given the same arguments). If
                /// the function is nondeterministic, call
                /// `register_nondeterministic_impl` instead.
                pub fn register_impl<F, Ret, #(#arg_name,)*>(
                    conn: &mut SqliteConnection,
                    f: F,
                ) -> QueryResult<()>
                where
                    F: Fn(#(#arg_name,)*) -> Ret + std::panic::UnwindSafe + Send + 'static,
                    (#(#arg_name,)*): FromSqlRow<(#(#arg_type,)*), Sqlite> +
                        StaticallySizedRow<(#(#arg_type,)*), Sqlite>,
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
                    conn: &mut SqliteConnection,
                    mut f: F,
                ) -> QueryResult<()>
                where
                    F: FnMut(#(#arg_name,)*) -> Ret + std::panic::UnwindSafe + Send + 'static,
                    (#(#arg_name,)*): FromSqlRow<(#(#arg_type,)*), Sqlite> +
                        StaticallySizedRow<(#(#arg_type,)*), Sqlite>,
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

        if is_supported_on_sqlite && arg_name.is_empty() {
            tokens = quote! {
                #tokens

                use diesel::sqlite::{Sqlite, SqliteConnection};
                use diesel::serialize::ToSql;

                #[allow(dead_code)]
                /// Registers an implementation for this function on the given connection
                ///
                /// This function must be called for every `SqliteConnection` before
                /// this SQL function can be used on SQLite. The implementation must be
                /// deterministic (returns the same result given the same arguments). If
                /// the function is nondeterministic, call
                /// `register_nondeterministic_impl` instead.
                pub fn register_impl<F, Ret>(
                    conn: &SqliteConnection,
                    f: F,
                ) -> QueryResult<()>
                where
                    F: Fn() -> Ret + std::panic::UnwindSafe + Send + 'static,
                    Ret: ToSql<#return_type, Sqlite>,
                {
                    conn.register_noarg_sql_function::<#return_type, _, _>(
                        #sql_name,
                        true,
                        f,
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
                pub fn register_nondeterministic_impl<F, Ret>(
                    conn: &SqliteConnection,
                    mut f: F,
                ) -> QueryResult<()>
                where
                    F: FnMut() -> Ret + std::panic::UnwindSafe + Send + 'static,
                    Ret: ToSql<#return_type, Sqlite>,
                {
                    conn.register_noarg_sql_function::<#return_type, _, _>(
                        #sql_name,
                        false,
                        f,
                    )
                }
            };
        }
    }

    let args_iter = args.iter();

    quote! {
        #(#attributes)*
        #[allow(non_camel_case_types)]
        pub #fn_token #fn_name #impl_generics (#(#args_iter,)*)
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
    }
}

pub(crate) struct SqlFunctionDecl {
    attributes: Vec<Attribute>,
    fn_token: Token![fn],
    fn_name: Ident,
    generics: Generics,
    args: Punctuated<StrictFnArg, Token![,]>,
    return_type: Type,
}

impl Parse for SqlFunctionDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        let attributes = Attribute::parse_outer(input)?;
        let fn_token: Token![fn] = input.parse()?;
        let fn_name = Ident::parse(input)?;
        let generics = Generics::parse(input)?;
        let args;
        let _paren = parenthesized!(args in input);
        let args = args.parse_terminated::<_, Token![,]>(StrictFnArg::parse)?;
        let return_type = if Option::<Token![->]>::parse(input)?.is_some() {
            Type::parse(input)?
        } else {
            parse_quote!(diesel::expression::expression_types::NotSelectable)
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

/// Essentially the same as ArgCaptured, but only allowing ident patterns
struct StrictFnArg {
    name: Ident,
    colon_token: Token![:],
    ty: Type,
}

impl Parse for StrictFnArg {
    fn parse(input: ParseStream) -> Result<Self> {
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

fn is_sqlite_type(ty: &Type) -> bool {
    let last_segment = if let Type::Path(tp) = ty {
        if let Some(segment) = tp.path.segments.last() {
            segment
        } else {
            return false;
        }
    } else {
        return false;
    };

    let ident = last_segment.ident.to_string();
    if ident == "Nullable" {
        if let PathArguments::AngleBracketed(ref ab) = last_segment.arguments {
            if let Some(GenericArgument::Type(ty)) = ab.args.first() {
                return is_sqlite_type(&ty);
            }
        }
        return false;
    }

    [
        "BigInt",
        "Binary",
        "Bool",
        "Date",
        "Double",
        "Float",
        "Integer",
        "Numeric",
        "SmallInt",
        "Text",
        "Time",
        "Timestamp",
    ]
    .contains(&ident.as_str())
}
