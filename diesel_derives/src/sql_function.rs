use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parenthesized, parse_quote, Attribute, GenericArgument, Generics, Ident, ImplGenerics, LitStr,
    PathArguments, Token, Type, TypeGenerics,
};

use crate::attrs::{AttributeSpanWrapper, MySpanned};
use crate::util::parse_eq;

pub(crate) fn expand(
    input: SqlFunctionDecl,
    legacy_helper_type_and_module: bool,
) -> Result<TokenStream> {
    let SqlFunctionDecl {
        attributes,
        fn_token,
        fn_name,
        mut generics,
        ref args,
        return_type,
    } = input;

    let sql_name = attributes
        .iter()
        .find_map(|attr| match attr.item {
            SqlFunctionAttribute::SqlName(_, ref value) => Some(value.value()),
            _ => None,
        })
        .unwrap_or_else(|| fn_name.to_string());

    let is_aggregate = attributes
        .iter()
        .any(|attr| matches!(attr.item, SqlFunctionAttribute::Aggregate(..)));

    let can_be_called_directly = !function_cannot_be_called_directly(&attributes)?;

    let window = attributes
        .iter()
        .find(|a| matches!(a.item, SqlFunctionAttribute::Window(..)))
        .cloned();

    let restrictions = attributes
        .iter()
        .find_map(|a| match a.item {
            SqlFunctionAttribute::Restriction(ref r) => Some(r.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let attributes = attributes
        .into_iter()
        .filter_map(|a| match a.item {
            SqlFunctionAttribute::Other(a) => Some(a),
            _ => None,
        })
        .collect::<Vec<_>>();

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

    let helper_type_doc = format!("The return type of [`{fn_name}()`](super::fn_name)");
    let query_fragment_impl =
        can_be_called_directly.then_some(restrictions.generate_all_queryfragment_impls(
            generics.clone(),
            &ty_generics,
            arg_name,
            &fn_name,
        ));

    let args_iter = args.iter();
    let mut tokens = quote! {
        use diesel::{self, QueryResult};
        use diesel::expression::{AsExpression, Expression, SelectableExpression, AppearsOnTable, ValidGrouping};
        use diesel::query_builder::{QueryFragment, AstPass};
        use diesel::sql_types::*;
        use diesel::internal::sql_functions::*;
        use super::*;

        #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId)]
        #numeric_derive
        pub struct #fn_name #ty_generics {
            #(pub(in super) #args_iter,)*
            #(pub(in super) #type_args: ::std::marker::PhantomData<#type_args>,)*
        }

        #[doc = #helper_type_doc]
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

        impl #impl_generics_internal FunctionFragment<__DieselInternal>
            for #fn_name #ty_generics
        where
            __DieselInternal: diesel::backend::Backend,
            #(#arg_name: QueryFragment<__DieselInternal>,)*
        {
            const FUNCTION_NAME: &'static str = #sql_name;

            #[allow(unused_assignments)]
            fn walk_arguments<'__b>(&'__b self, mut out: AstPass<'_, '__b, __DieselInternal>) -> QueryResult<()> {
                // we unroll the arguments manually here, to prevent borrow check issues
                let mut needs_comma = false;
                #(
                    if !self.#arg_name.is_noop(out.backend())? {
                        if needs_comma {
                            out.push_sql(", ");
                        }
                        self.#arg_name.walk_ast(out.reborrow())?;
                        needs_comma = true;
                    }
                )*
                Ok(())
            }
        }

        #query_fragment_impl
    };

    let is_supported_on_sqlite = cfg!(feature = "sqlite")
        && type_args.is_empty()
        && is_sqlite_type(&return_type)
        && arg_type.iter().all(|a| is_sqlite_type(a));

    if let Some(ref window) = window {
        tokens = generate_window_function_tokens(
            window,
            &impl_generics,
            generics.clone(),
            &ty_generics,
            &fn_name,
            arg_name,
            tokens,
        );
    }

    if is_aggregate {
        tokens = generate_tokens_for_aggregate_functions(
            tokens,
            &impl_generics_internal,
            &impl_generics,
            &fn_name,
            &ty_generics,
            arg_name,
            arg_type,
            is_supported_on_sqlite,
            window.as_ref(),
            &return_type,
            &sql_name,
        );
    } else if window.is_none() {
        tokens = generate_tokens_for_non_aggregate_functions(
            tokens,
            &impl_generics_internal,
            &fn_name,
            &ty_generics,
            arg_name,
            arg_type,
            is_supported_on_sqlite,
            &return_type,
            &sql_name,
        );
    }

    let args_iter = args.iter();

    let (outside_of_module_helper_type, return_type_path, internals_module_name) =
        if legacy_helper_type_and_module {
            (None, quote! { #fn_name::HelperType }, fn_name.clone())
        } else {
            let internals_module_name = Ident::new(&format!("{fn_name}_utils"), fn_name.span());
            (
                Some(quote! {
                    #[allow(non_camel_case_types, non_snake_case)]
                    #[doc = #helper_type_doc]
                    pub type #fn_name #ty_generics = #internals_module_name::#fn_name <
                        #(#type_args,)*
                        #(<#arg_name as diesel::expression::AsExpression<#arg_type>>::Expression,)*
                    >;
                }),
                quote! { #fn_name },
                internals_module_name,
            )
        };

    Ok(quote! {
        #(#attributes)*
        #[allow(non_camel_case_types)]
        pub #fn_token #fn_name #impl_generics (#(#args_iter,)*)
            -> #return_type_path #ty_generics
        #where_clause
            #(#arg_name: diesel::expression::AsExpression<#arg_type>,)*
        {
            #internals_module_name::#fn_name {
                #(#arg_struct_assign,)*
                #(#type_args: ::std::marker::PhantomData,)*
            }
        }

        #outside_of_module_helper_type

        #[doc(hidden)]
        #[allow(non_camel_case_types, non_snake_case, unused_imports)]
        pub(crate) mod #internals_module_name {
            #tokens
        }
    })
}

fn generate_window_function_tokens(
    window: &AttributeSpanWrapper<SqlFunctionAttribute>,
    impl_generics: &syn::ImplGenerics<'_>,
    generics: Generics,
    ty_generics: &TypeGenerics<'_>,
    fn_name: &Ident,
    arg_name: &[&syn::Ident],
    tokens: TokenStream,
) -> TokenStream {
    let SqlFunctionAttribute::Window(_, ref restrictions) = window.item else {
        unreachable!("We filtered for window attributes above")
    };
    let window_function_impl =
        restrictions.generate_all_window_fragment_impls(generics, ty_generics, fn_name);
    quote::quote! {
        #tokens
        #window_function_impl
        impl #impl_generics IsWindowFunction for #fn_name #ty_generics {
            type ArgTypes = (#(#arg_name,)*);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_tokens_for_non_aggregate_functions(
    mut tokens: TokenStream,
    impl_generics_internal: &syn::ImplGenerics<'_>,
    fn_name: &syn::Ident,
    ty_generics: &syn::TypeGenerics<'_>,
    arg_name: &[&syn::Ident],
    arg_type: &[&syn::Type],
    is_supported_on_sqlite: bool,
    return_type: &syn::Type,
    sql_name: &str,
) -> TokenStream {
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
    tokens
}

#[allow(clippy::too_many_arguments)]
fn generate_tokens_for_aggregate_functions(
    mut tokens: TokenStream,
    impl_generics_internal: &syn::ImplGenerics<'_>,
    impl_generics: &syn::ImplGenerics<'_>,
    fn_name: &syn::Ident,
    ty_generics: &syn::TypeGenerics<'_>,
    arg_name: &[&syn::Ident],
    arg_type: &[&syn::Type],
    is_supported_on_sqlite: bool,
    window: Option<&AttributeSpanWrapper<SqlFunctionAttribute>>,
    return_type: &syn::Type,
    sql_name: &str,
) -> TokenStream {
    tokens = quote! {
        #tokens

        impl #impl_generics_internal ValidGrouping<__DieselInternal>
            for #fn_name #ty_generics
        {
            type IsAggregate = diesel::expression::is_aggregate::Yes;
        }

        impl #impl_generics IsAggregateFunction for #fn_name #ty_generics {}
    };
    // we do not support custom window functions for sqlite yet
    if is_supported_on_sqlite && window.is_none() {
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
            1 => {
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
    tokens
}

fn function_cannot_be_called_directly(
    attributes: &[AttributeSpanWrapper<SqlFunctionAttribute>],
) -> Result<bool> {
    let mut has_aggregate = false;
    let mut has_window = false;
    let mut has_require_within = false;
    for attr in attributes {
        has_aggregate = has_aggregate || matches!(attr.item, SqlFunctionAttribute::Aggregate(..));
        has_window = has_window || matches!(attr.item, SqlFunctionAttribute::Window(..));
        has_require_within =
            has_require_within || matches!(attr.item, SqlFunctionAttribute::RequireWithin(..));
        if has_require_within && (has_aggregate || has_window) {
            return Err(syn::Error::new(attr.ident_span, "cannot have `#[require_within]` and `#[aggregate]` or `#[window]` on the same function"));
        }
    }
    Ok(has_require_within || (has_window && !has_aggregate))
}

pub(crate) struct SqlFunctionDecl {
    attributes: Vec<AttributeSpanWrapper<SqlFunctionAttribute>>,
    fn_token: Token![fn],
    fn_name: Ident,
    generics: Generics,
    args: Punctuated<StrictFnArg, Token![,]>,
    return_type: Type,
}

impl Parse for SqlFunctionDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        let attributes = Attribute::parse_outer(input)?;

        let attributes = attributes
            .into_iter()
            .map(|attr| match &attr.meta {
                syn::Meta::NameValue(syn::MetaNameValue {
                    path,
                    value:
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(sql_name),
                            ..
                        }),
                    ..
                }) if path.is_ident("sql_name") => Ok(AttributeSpanWrapper {
                    attribute_span: attr.span(),
                    ident_span: sql_name.span(),
                    item: SqlFunctionAttribute::SqlName(
                        path.require_ident()?.clone(),
                        sql_name.clone(),
                    ),
                }),
                syn::Meta::Path(path) if path.is_ident("aggregate") => Ok(AttributeSpanWrapper {
                    attribute_span: attr.span(),
                    ident_span: path.span(),
                    item: SqlFunctionAttribute::Aggregate(path.require_ident()?.clone()),
                }),
                syn::Meta::Path(path) if path.is_ident("window") => Ok(AttributeSpanWrapper {
                    attribute_span: attr.span(),
                    ident_span: path.span(),
                    item: SqlFunctionAttribute::Window(
                        path.require_ident()?.clone(),
                        BackendRestriction::None,
                    ),
                }),
                syn::Meta::Path(path) if path.is_ident("require_within") => {
                    Ok(AttributeSpanWrapper {
                        attribute_span: attr.span(),
                        ident_span: path.span(),
                        item: SqlFunctionAttribute::RequireWithin(path.require_ident()?.clone()),
                    })
                }
                syn::Meta::NameValue(_) | syn::Meta::Path(_) => Ok(AttributeSpanWrapper {
                    attribute_span: attr.span(),
                    ident_span: attr.span(),
                    item: SqlFunctionAttribute::Other(attr),
                }),
                syn::Meta::List(_) => {
                    let name = attr.meta.path().require_ident()?;
                    let attribute_span = attr.meta.span();
                    attr.clone()
                        .parse_args_with(|input: &syn::parse::ParseBuffer| {
                            SqlFunctionAttribute::parse_attr(
                                name.clone(),
                                input,
                                attr.clone(),
                                attribute_span,
                            )
                        })
                }
            })
            .collect::<Result<Vec<_>>>()?;

        let fn_token: Token![fn] = input.parse()?;
        let fn_name = Ident::parse(input)?;
        let generics = Generics::parse(input)?;
        let args;
        let _paren = parenthesized!(args in input);
        let args = args.parse_terminated(StrictFnArg::parse, Token![,])?;
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
                return is_sqlite_type(ty);
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

#[derive(Default, Clone, Debug)]
enum BackendRestriction {
    #[default]
    None,
    SqlDialect(syn::Ident, syn::Ident, syn::Path),
    BackendBound(
        syn::Ident,
        syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
    ),
    Backends(
        syn::Ident,
        syn::punctuated::Punctuated<syn::Path, syn::Token![,]>,
    ),
}

impl BackendRestriction {
    fn parse_from(input: &syn::parse::ParseBuffer<'_>) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self::None);
        }
        Self::parse(input)
    }

    fn parse_backends(
        input: &syn::parse::ParseBuffer<'_>,
        name: Ident,
    ) -> Result<BackendRestriction> {
        let backends = Punctuated::parse_terminated(input)?;
        Ok(Self::Backends(name, backends))
    }

    fn parse_sql_dialect(
        content: &syn::parse::ParseBuffer<'_>,
        name: Ident,
    ) -> Result<BackendRestriction> {
        let dialect = content.parse()?;
        let _del: syn::Token![,] = content.parse()?;
        let dialect_variant = content.parse()?;

        Ok(Self::SqlDialect(name, dialect, dialect_variant))
    }

    fn parse_backend_bounds(
        input: &syn::parse::ParseBuffer<'_>,
        name: Ident,
    ) -> Result<BackendRestriction> {
        let restrictions = Punctuated::parse_terminated(input)?;
        Ok(Self::BackendBound(name, restrictions))
    }

    fn generate_all_window_fragment_impls(
        &self,
        mut generics: Generics,
        ty_generics: &TypeGenerics<'_>,
        fn_name: &syn::Ident,
    ) -> TokenStream {
        generics.params.push(parse_quote!(__P));
        generics.params.push(parse_quote!(__O));
        generics.params.push(parse_quote!(__F));
        match *self {
            BackendRestriction::None => {
                generics.params.push(parse_quote!(__DieselInternal));
                let (impl_generics, _, _) = generics.split_for_impl();
                Self::generate_window_fragment_impl(
                    parse_quote!(__DieselInternal),
                    Some(parse_quote!(__DieselInternal: diesel::backend::Backend,)),
                    &impl_generics,
                    ty_generics,
                    fn_name,
                    None,
                )
            }
            BackendRestriction::SqlDialect(_, ref dialect, ref dialect_type) => {
                generics.params.push(parse_quote!(__DieselInternal));
                let (impl_generics, _, _) = generics.split_for_impl();
                let specific_impl = Self::generate_window_fragment_impl(
                    parse_quote!(__DieselInternal),
                    Some(
                        parse_quote!(__DieselInternal: diesel::backend::Backend + diesel::backend::SqlDialect<#dialect = #dialect_type>,),
                    ),
                    &impl_generics,
                    ty_generics,
                    fn_name,
                    Some(dialect_type),
                );
                quote::quote! {
                    impl #impl_generics WindowFunctionFragment<__DieselInternal>
                        for #fn_name #ty_generics
                    where
                        Self: WindowFunctionFragment<__DieselInternal, <__DieselInternal as diesel::backend::SqlDialect>::#dialect>,
                        __DieselInternal: diesel::backend::Backend,
                    {
                    }

                    #specific_impl
                }
            }
            BackendRestriction::BackendBound(_, ref restriction) => {
                generics.params.push(parse_quote!(__DieselInternal));
                let (impl_generics, _, _) = generics.split_for_impl();
                Self::generate_window_fragment_impl(
                    parse_quote!(__DieselInternal),
                    Some(parse_quote!(__DieselInternal: diesel::backend::Backend + #restriction,)),
                    &impl_generics,
                    ty_generics,
                    fn_name,
                    None,
                )
            }
            BackendRestriction::Backends(_, ref backends) => {
                let (impl_generics, _, _) = generics.split_for_impl();
                let backends = backends.iter().map(|b| {
                    Self::generate_window_fragment_impl(
                        quote! {#b},
                        None,
                        &impl_generics,
                        ty_generics,
                        fn_name,
                        None,
                    )
                });

                parse_quote!(#(#backends)*)
            }
        }
    }

    fn generate_window_fragment_impl(
        backend: TokenStream,
        backend_bound: Option<proc_macro2::TokenStream>,
        impl_generics: &ImplGenerics<'_>,
        ty_generics: &TypeGenerics<'_>,
        fn_name: &syn::Ident,
        dialect: Option<&syn::Path>,
    ) -> TokenStream {
        quote::quote! {
            impl #impl_generics WindowFunctionFragment<#fn_name #ty_generics, #backend, #dialect> for OverClause<__P, __O, __F>
                where #backend_bound
            {

            }
        }
    }

    fn generate_all_queryfragment_impls(
        &self,
        mut generics: Generics,
        ty_generics: &TypeGenerics<'_>,
        arg_name: &[&syn::Ident],
        fn_name: &syn::Ident,
    ) -> proc_macro2::TokenStream {
        match *self {
            BackendRestriction::None => {
                generics.params.push(parse_quote!(__DieselInternal));
                let (impl_generics, _, _) = generics.split_for_impl();
                Self::generate_queryfragment_impl(
                    parse_quote!(__DieselInternal),
                    Some(parse_quote!(__DieselInternal: diesel::backend::Backend,)),
                    &impl_generics,
                    ty_generics,
                    arg_name,
                    fn_name,
                    None,
                )
            }
            BackendRestriction::BackendBound(_, ref restriction) => {
                generics.params.push(parse_quote!(__DieselInternal));
                let (impl_generics, _, _) = generics.split_for_impl();
                Self::generate_queryfragment_impl(
                    parse_quote!(__DieselInternal),
                    Some(parse_quote!(__DieselInternal: diesel::backend::Backend + #restriction,)),
                    &impl_generics,
                    ty_generics,
                    arg_name,
                    fn_name,
                    None,
                )
            }
            BackendRestriction::SqlDialect(_, ref dialect, ref dialect_type) => {
                generics.params.push(parse_quote!(__DieselInternal));
                let (impl_generics, _, _) = generics.split_for_impl();
                let specific_impl = Self::generate_queryfragment_impl(
                    parse_quote!(__DieselInternal),
                    Some(
                        parse_quote!(__DieselInternal: diesel::backend::Backend + diesel::backend::SqlDialect<#dialect = #dialect_type>,),
                    ),
                    &impl_generics,
                    ty_generics,
                    arg_name,
                    fn_name,
                    Some(dialect_type),
                );
                quote::quote! {
                    impl #impl_generics QueryFragment<__DieselInternal>
                        for #fn_name #ty_generics
                    where
                        Self: QueryFragment<__DieselInternal, <__DieselInternal as diesel::backend::SqlDialect>::#dialect>,
                        __DieselInternal: diesel::backend::Backend,
                    {
                        fn walk_ast<'__b>(&'__b self, mut out: AstPass<'_, '__b, __DieselInternal>) -> QueryResult<()> {
                            <Self as QueryFragment<__DieselInternal, <__DieselInternal as diesel::backend::SqlDialect>::#dialect>>::walk_ast(self, out)
                        }

                    }

                    #specific_impl
                }
            }
            BackendRestriction::Backends(_, ref backends) => {
                let (impl_generics, _, _) = generics.split_for_impl();
                let backends = backends.iter().map(|b| {
                    Self::generate_queryfragment_impl(
                        quote! {#b},
                        None,
                        &impl_generics,
                        ty_generics,
                        arg_name,
                        fn_name,
                        None,
                    )
                });

                parse_quote!(#(#backends)*)
            }
        }
    }

    fn generate_queryfragment_impl(
        backend: proc_macro2::TokenStream,
        backend_bound: Option<proc_macro2::TokenStream>,
        impl_generics: &ImplGenerics<'_>,
        ty_generics: &TypeGenerics<'_>,
        arg_name: &[&syn::Ident],
        fn_name: &syn::Ident,
        dialect: Option<&syn::Path>,
    ) -> proc_macro2::TokenStream {
        quote::quote! {
            impl #impl_generics QueryFragment<#backend, #dialect>
                for #fn_name #ty_generics
            where
                #backend_bound
            #(#arg_name: QueryFragment<#backend>,)*
            {
                fn walk_ast<'__b>(&'__b self, mut out: AstPass<'_, '__b, #backend>) -> QueryResult<()>{
                    out.push_sql(<Self as FunctionFragment<#backend>>::FUNCTION_NAME);
                    out.push_sql("(");
                    self.walk_arguments(out.reborrow())?;
                    out.push_sql(")");
                    Ok(())
                }
            }
        }
    }
}

impl Parse for BackendRestriction {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: syn::Ident = input.parse()?;
        let name_str = name.to_string();
        let content;
        parenthesized!(content in input);
        match &*name_str {
            "backends" => Self::parse_backends(&content, name),
            "dialect" => Self::parse_sql_dialect(&content, name),
            "backend_bounds" => Self::parse_backend_bounds(&content, name),
            _ => Err(syn::Error::new(
                name.span(),
                format!("unexpected option `{name_str}`"),
            )),
        }
    }
}

#[derive(Debug, Clone)]
enum SqlFunctionAttribute {
    Aggregate(Ident),
    Window(Ident, BackendRestriction),
    SqlName(Ident, LitStr),
    Restriction(BackendRestriction),
    RequireWithin(Ident),
    Other(Attribute),
}

impl MySpanned for SqlFunctionAttribute {
    fn span(&self) -> proc_macro2::Span {
        match self {
            SqlFunctionAttribute::Restriction(BackendRestriction::Backends(ref ident, ..))
            | SqlFunctionAttribute::Restriction(BackendRestriction::SqlDialect(ref ident, ..))
            | SqlFunctionAttribute::Restriction(BackendRestriction::BackendBound(ref ident, ..))
            | SqlFunctionAttribute::Aggregate(ref ident, ..)
            | SqlFunctionAttribute::Window(ref ident, ..)
            | SqlFunctionAttribute::RequireWithin(ref ident)
            | SqlFunctionAttribute::SqlName(ref ident, ..) => ident.span(),
            SqlFunctionAttribute::Restriction(BackendRestriction::None) => {
                unreachable!("We do not construct that")
            }
            SqlFunctionAttribute::Other(ref attribute) => attribute.span(),
        }
    }
}

impl SqlFunctionAttribute {
    fn parse_attr(
        name: Ident,
        input: &syn::parse::ParseBuffer<'_>,
        attr: Attribute,
        attribute_span: proc_macro2::Span,
    ) -> Result<AttributeSpanWrapper<Self>> {
        let name_str = name.to_string();
        let parsed_attr = match &*name_str {
            "window" => BackendRestriction::parse_from(input).map(|r| Self::Window(name, r))?,
            "sql_name" => parse_eq(input, "sql_name = \"SUM\"").map(|v| Self::SqlName(name, v))?,
            "backends" => BackendRestriction::parse_backends(input, name).map(Self::Restriction)?,
            "dialect" => {
                BackendRestriction::parse_sql_dialect(input, name).map(Self::Restriction)?
            }
            "backend_bounds" => {
                BackendRestriction::parse_backend_bounds(input, name).map(Self::Restriction)?
            }
            _ => {
                // empty the parse buffer otherwise syn will return an error
                let _ = input.step(|cursor| {
                    let mut rest = *cursor;
                    while let Some((_, next)) = rest.token_tree() {
                        rest = next;
                    }
                    Ok(((), rest))
                });
                SqlFunctionAttribute::Other(attr)
            }
        };
        Ok(AttributeSpanWrapper {
            ident_span: parsed_attr.span(),
            item: parsed_attr,
            attribute_span,
        })
    }
}
