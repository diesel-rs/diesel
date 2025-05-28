use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::ToTokens;
use quote::TokenStreamExt;
use std::iter;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Pair;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::GenericParam;
use syn::LitInt;
use syn::Path;
use syn::{
    parenthesized, parse_quote, Attribute, GenericArgument, Generics, Ident, Meta, MetaNameValue,
    PathArguments, Token, Type,
};

const VARIADIC_VARIANTS_DEFAULT: usize = 2;
const VARIADIC_ARG_COUNT_ENV: Option<&str> = option_env!("DIESEL_VARIADIC_FUNCTION_ARGS");

pub(crate) struct Expanded {
    pub tokens: TokenStream,
    pub return_type_helpers: TokenStream,
}

pub(crate) fn expand(input: Vec<SqlFunctionDecl>, legacy_helper_type_and_module: bool) -> Expanded {
    let mut result = TokenStream::new();
    let mut return_type_helper_module_paths = vec![];

    for decl in input {
        let expanded = expand_one(decl, legacy_helper_type_and_module);
        let expanded = match expanded {
            Err(err) => err.into_compile_error(),
            Ok(expanded) => {
                if let Some(return_type_helper_module_path) =
                    expanded.return_type_helper_module_path
                {
                    return_type_helper_module_paths.push(return_type_helper_module_path);
                }

                expanded.tokens
            }
        };

        result.append_all(expanded.into_iter());
    }

    let return_type_helpers = quote! {
        #[allow(unused_imports)]
        pub mod return_type_helpers {
            #(
                #[doc(inline)]
                pub use super:: #return_type_helper_module_paths ::*;
            )*
        }
    };

    Expanded {
        tokens: result,
        return_type_helpers,
    }
}

struct ExpandedSqlFunction {
    tokens: TokenStream,
    return_type_helper_module_path: Option<Path>,
}

fn expand_one(
    mut input: SqlFunctionDecl,
    legacy_helper_type_and_module: bool,
) -> syn::Result<ExpandedSqlFunction> {
    let attributes = &mut input.attributes;

    let variadic_argument_count = attributes
        .iter()
        .find(|attr| attr.meta.path().is_ident("variadic"))
        .map(|attr| {
            let argument_count_literal = attr.parse_args::<VariadicAttributeArgs>()?.argument_count;
            let argument_count = argument_count_literal.base10_parse::<usize>()?;

            if argument_count > input.args.len() {
                return Err(syn::Error::new(
                    argument_count_literal.span(),
                    "invalid variadic argument count: not enough function arguments",
                ));
            }

            Ok(argument_count)
        });

    let Some(variadic_argument_count) = variadic_argument_count else {
        let sql_name = parse_sql_name_attr(&mut input).unwrap_or_else(|| input.fn_name.to_string());

        return expand_nonvariadic(input, sql_name, legacy_helper_type_and_module);
    };

    let variadic_argument_count = variadic_argument_count?;

    attributes.retain(|attr| !attr.meta.path().is_ident("variadic"));

    let variadic_variants = VARIADIC_ARG_COUNT_ENV
        .and_then(|arg_count| arg_count.parse::<usize>().ok())
        .unwrap_or(VARIADIC_VARIANTS_DEFAULT);

    let mut result = TokenStream::new();
    let mut helper_type_modules = vec![];
    for variant_no in 0..=variadic_variants {
        let expanded = expand_variadic(
            input.clone(),
            legacy_helper_type_and_module,
            variadic_argument_count,
            variant_no,
        )?;

        if let Some(return_type_helper_module_path) = expanded.return_type_helper_module_path {
            helper_type_modules.push(return_type_helper_module_path);
        }

        result.append_all(expanded.tokens.into_iter());
    }

    let return_types_module_name = Ident::new(
        &format!("__{}_return_types", input.fn_name),
        input.fn_name.span(),
    );
    let result = quote! {
        #result

        #[allow(unused_imports)]
        mod #return_types_module_name {
            #(
                #[doc(inline)]
                pub use super:: #helper_type_modules ::*;
            )*
        }
    };

    let return_type_helper_module_path = Some(parse_quote! {
        #return_types_module_name
    });

    Ok(ExpandedSqlFunction {
        tokens: result,
        return_type_helper_module_path,
    })
}

fn expand_variadic(
    mut input: SqlFunctionDecl,
    legacy_helper_type_and_module: bool,
    variadic_argument_count: usize,
    variant_no: usize,
) -> syn::Result<ExpandedSqlFunction> {
    add_variadic_doc_comments(&mut input.attributes, &input.fn_name.to_string());

    let sql_name = parse_sql_name_attr(&mut input).unwrap_or_else(|| input.fn_name.to_string());

    input.fn_name = format_ident!("{}_{}", input.fn_name, variant_no);

    let nonvariadic_args_count = input.args.len() - variadic_argument_count;

    let mut variadic_generic_indexes = vec![];
    let mut arguments_with_generic_types = vec![];
    for (arg_idx, arg) in input.args.iter().skip(nonvariadic_args_count).enumerate() {
        // If argument is of type that definitely cannot be a generic then we skip it.
        let Type::Path(ty_path) = arg.ty.clone() else {
            continue;
        };
        let Ok(ty_ident) = ty_path.path.require_ident() else {
            continue;
        };

        let idx = input.generics.params.iter().position(|param| match param {
            GenericParam::Type(type_param) => type_param.ident == *ty_ident,
            _ => false,
        });

        if let Some(idx) = idx {
            variadic_generic_indexes.push(idx);
            arguments_with_generic_types.push(arg_idx);
        }
    }

    let mut args: Vec<_> = input.args.into_pairs().collect();
    let variadic_args = args.split_off(nonvariadic_args_count);
    let nonvariadic_args = args;

    let variadic_args: Vec<_> = iter::repeat_n(variadic_args, variant_no)
        .enumerate()
        .flat_map(|(arg_group_idx, arg_group)| {
            let mut resulting_args = vec![];

            for (arg_idx, arg) in arg_group.into_iter().enumerate() {
                let mut arg = arg.into_value();

                arg.name = format_ident!("{}_{}", arg.name, arg_group_idx + 1);

                if arguments_with_generic_types.contains(&arg_idx) {
                    let Type::Path(mut ty_path) = arg.ty.clone() else {
                        unreachable!("This argument should have path type as checked earlier")
                    };
                    let Ok(ident) = ty_path.path.require_ident() else {
                        unreachable!("This argument should have ident type as checked earlier")
                    };

                    ty_path.path.segments[0].ident =
                        format_ident!("{}{}", ident, arg_group_idx + 1);
                    arg.ty = Type::Path(ty_path);
                }

                let pair = Pair::new(arg, Some(Token![,]([Span::call_site()])));
                resulting_args.push(pair);
            }

            resulting_args
        })
        .collect();

    input.args = nonvariadic_args.into_iter().chain(variadic_args).collect();

    let generics: Vec<_> = input.generics.params.into_pairs().collect();
    input.generics.params = if variant_no == 0 {
        generics
            .into_iter()
            .enumerate()
            .filter_map(|(generic_idx, generic)| {
                (!variadic_generic_indexes.contains(&generic_idx)).then_some(generic)
            })
            .collect()
    } else {
        iter::repeat_n(generics, variant_no)
            .enumerate()
            .flat_map(|(generic_group_idx, generic_group)| {
                let mut resulting_generics = vec![];

                for (generic_idx, generic) in generic_group.into_iter().enumerate() {
                    if !variadic_generic_indexes.contains(&generic_idx) {
                        if generic_group_idx == 0 {
                            resulting_generics.push(generic);
                        }

                        continue;
                    }

                    let mut generic = generic.into_value();

                    if let GenericParam::Type(type_param) = &mut generic {
                        type_param.ident =
                            format_ident!("{}{}", type_param.ident, generic_group_idx + 1);
                    } else {
                        unreachable!("This generic should be a type param as checked earlier")
                    }

                    let pair = Pair::new(generic, Some(Token![,]([Span::call_site()])));
                    resulting_generics.push(pair);
                }

                resulting_generics
            })
            .collect()
    };

    expand_nonvariadic(input, sql_name, legacy_helper_type_and_module)
}

fn add_variadic_doc_comments(attributes: &mut Vec<Attribute>, fn_name: &str) {
    let mut doc_comments_end = attributes.len()
        - attributes
            .iter()
            .rev()
            .position(|attr| match &attr.meta {
                Meta::NameValue(MetaNameValue { path, .. }) => path.is_ident("doc"),
                _ => false,
            })
            .unwrap_or(attributes.len());

    let fn_family = format!("`{0}_0`, `{0}_1`, ... `{0}_n`", fn_name);

    let doc_comments: Vec<Attribute> = parse_quote! {
        ///
        /// # Variadic functions
        ///
        /// This function is variadic in SQL, so there's a family of functions
        /// on a diesel side:
        ///
        #[doc = #fn_family]
        ///
        /// Here, the postfix number indicates repetitions of variadic arguments.
        /// To use this function, the appropriate version with the correct
        /// argument count must be selected.
        ///
        /// ## Controlling the generation of variadic function variants
        ///
        /// By default, only variants with 0, 1, and 2 repetitions of variadic
        /// arguments are generated. To generate more variants, set the
        /// `DIESEL_VARIADIC_FUNCTION_ARGS` environment variable to the desired
        /// number of variants.
        #[doc(alias = #fn_name)]
    };

    for new_attribute in doc_comments {
        attributes.insert(doc_comments_end, new_attribute);
        doc_comments_end += 1;
    }
}

fn parse_sql_name_attr(input: &mut SqlFunctionDecl) -> Option<String> {
    let result = input
        .attributes
        .iter()
        .find(|attr| attr.meta.path().is_ident("sql_name"))
        .and_then(|attr| {
            if let Meta::NameValue(MetaNameValue {
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(ref lit),
                        ..
                    }),
                ..
            }) = attr.meta
            {
                Some(lit.value())
            } else {
                None
            }
        });

    input
        .attributes
        .retain(|attr| !attr.meta.path().is_ident("sql_name"));

    result
}

fn expand_nonvariadic(
    input: SqlFunctionDecl,
    sql_name: String,
    legacy_helper_type_and_module: bool,
) -> syn::Result<ExpandedSqlFunction> {
    let SqlFunctionDecl {
        mut attributes,
        fn_token,
        fn_name,
        mut generics,
        args,
        return_type,
    } = input;

    let is_aggregate = attributes
        .iter()
        .any(|attr| attr.meta.path().is_ident("aggregate"));
    attributes.retain(|attr| !attr.meta.path().is_ident("aggregate"));

    let skip_return_type_helper = attributes
        .iter()
        .any(|attr| attr.meta.path().is_ident("skip_return_type_helper"));
    attributes.retain(|attr| !attr.meta.path().is_ident("skip_return_type_helper"));

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

    let helper_type_doc = format!("The return type of [`{fn_name}()`](super::fn_name)");

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

        // __DieselInternal is what we call DB normally
        impl #impl_generics_internal QueryFragment<__DieselInternal>
            for #fn_name #ty_generics
        where
            __DieselInternal: diesel::backend::Backend,
            #(#arg_name: QueryFragment<__DieselInternal>,)*
        {
            #[allow(unused_assignments)]
            fn walk_ast<'__b>(&'__b self, mut out: AstPass<'_, '__b, __DieselInternal>) -> QueryResult<()>{
                out.push_sql(concat!(#sql_name, "("));
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

    let (return_type_helper_module, return_type_helper_module_path) = if skip_return_type_helper {
        (None, None)
    } else {
        let auto_derived_types = type_args
            .iter()
            .map(|type_arg| {
                for arg in args {
                    let Type::Path(path) = &arg.ty else {
                        continue;
                    };

                    let Some(path_ident) = path.path.get_ident() else {
                        continue;
                    };

                    if path_ident == type_arg {
                        return Ok(arg.name.clone());
                    }
                }

                Err(syn::Error::new(
                    type_arg.span(),
                    "cannot find argument corresponding to the generic",
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        let arg_names_iter: Vec<_> = args.iter().map(|arg| arg.name.clone()).collect();

        let return_type_module_name =
            Ident::new(&format!("__{}_return_type", fn_name), fn_name.span());

        let doc = format!("Return type of [`{fn_name}()`](super::{fn_name}()).");
        let return_type_helper_module = quote! {
            #[allow(non_camel_case_types, non_snake_case, unused_imports)]
            mod #return_type_module_name {
                #[doc = #doc]
                pub type #fn_name<
                    #(#arg_names_iter,)*
                > = super::#fn_name<
                    #( <#auto_derived_types as diesel::expression::Expression>::SqlType, )*
                    #(#arg_names_iter,)*
                >;
            }
        };

        let module_path = parse_quote!(
            #return_type_module_name
        );

        (Some(return_type_helper_module), Some(module_path))
    };

    let tokens = quote! {
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

        #return_type_helper_module

        #[doc(hidden)]
        #[allow(non_camel_case_types, non_snake_case, unused_imports)]
        pub(crate) mod #internals_module_name {
            #tokens
        }
    };

    Ok(ExpandedSqlFunction {
        tokens,
        return_type_helper_module_path,
    })
}

pub(crate) struct ExternSqlBlock {
    pub(crate) function_decls: Vec<SqlFunctionDecl>,
}

impl Parse for ExternSqlBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let block = syn::ItemForeignMod::parse(input)?;
        if block.abi.name.as_ref().map(|n| n.value()) != Some("SQL".into()) {
            return Err(syn::Error::new(block.abi.span(), "expect `SQL` as ABI"));
        }
        if block.unsafety.is_some() {
            return Err(syn::Error::new(
                block.unsafety.unwrap().span(),
                "expect `SQL` function blocks to be safe",
            ));
        }
        let function_decls = block
            .items
            .into_iter()
            .map(|i| syn::parse2(quote! { #i }))
            .collect::<Result<Vec<_>>>()?;

        Ok(ExternSqlBlock { function_decls })
    }
}

#[derive(Clone)]
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
#[derive(Clone)]
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

struct VariadicAttributeArgs {
    argument_count: LitInt,
}

impl Parse for VariadicAttributeArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            argument_count: LitInt::parse(input)?,
        })
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
