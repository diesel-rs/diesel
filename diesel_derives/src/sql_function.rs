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
use syn::{
    parenthesized, parse_quote, Attribute, GenericArgument, Generics, Ident, ImplGenerics, LitStr,
    PathArguments, Token, Type, TypeGenerics,
};
use syn::{GenericParam, Meta};
use syn::{LitBool, Path};
use syn::{LitInt, MetaNameValue};

use crate::attrs::{AttributeSpanWrapper, MySpanned};
use crate::util::parse_eq;

const VARIADIC_VARIANTS_DEFAULT: usize = 2;
const VARIADIC_ARG_COUNT_ENV: Option<&str> = option_env!("DIESEL_VARIADIC_FUNCTION_ARGS");

pub(crate) fn expand(
    input: Vec<SqlFunctionDecl>,
    legacy_helper_type_and_module: bool,
    generate_return_type_helpers: bool,
) -> TokenStream {
    let mut result = TokenStream::new();
    let mut return_type_helper_module_paths = vec![];

    for decl in input {
        let expanded = expand_one(
            decl,
            legacy_helper_type_and_module,
            generate_return_type_helpers,
        );
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

    if !generate_return_type_helpers {
        return result;
    }

    quote! {
        #result

        #[allow(unused_imports)]
        #[doc(hidden)]
        mod return_type_helpers {
            #(
                #[doc(inline)]
                pub use super:: #return_type_helper_module_paths ::*;
            )*
        }
    }
}

struct ExpandedSqlFunction {
    tokens: TokenStream,
    return_type_helper_module_path: Option<Path>,
}

fn expand_one(
    mut input: SqlFunctionDecl,
    legacy_helper_type_and_module: bool,
    generate_return_type_helpers: bool,
) -> syn::Result<ExpandedSqlFunction> {
    let attributes = &mut input.attributes;

    let variadic_argument_count = attributes.iter().find_map(|attr| {
        if let SqlFunctionAttribute::Variadic(_, c) = &attr.item {
            Some((c.base10_parse(), c.span()))
        } else {
            None
        }
    });

    let Some((variadic_argument_count, variadic_span)) = variadic_argument_count else {
        let sql_name = parse_sql_name_attr(&mut input);

        return expand_nonvariadic(
            input,
            sql_name,
            legacy_helper_type_and_module,
            generate_return_type_helpers,
        );
    };

    let variadic_argument_count = variadic_argument_count?;

    let variadic_variants = VARIADIC_ARG_COUNT_ENV
        .and_then(|arg_count| arg_count.parse::<usize>().ok())
        .unwrap_or(VARIADIC_VARIANTS_DEFAULT);

    let mut result = TokenStream::new();
    let mut helper_type_modules = vec![];
    for variant_no in 0..=variadic_variants {
        let expanded = expand_variadic(
            input.clone(),
            legacy_helper_type_and_module,
            generate_return_type_helpers,
            variadic_argument_count,
            variant_no,
            variadic_span,
        )?;

        if let Some(return_type_helper_module_path) = expanded.return_type_helper_module_path {
            helper_type_modules.push(return_type_helper_module_path);
        }

        result.append_all(expanded.tokens.into_iter());
    }

    if generate_return_type_helpers {
        let return_types_module_name = Ident::new(
            &format!("__{}_return_types", input.fn_name),
            input.fn_name.span(),
        );
        let result = quote! {
            #result

            #[allow(unused_imports)]
            #[doc(inline)]
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
    } else {
        Ok(ExpandedSqlFunction {
            tokens: result,
            return_type_helper_module_path: None,
        })
    }
}

fn expand_variadic(
    mut input: SqlFunctionDecl,
    legacy_helper_type_and_module: bool,
    generate_return_type_helpers: bool,
    variadic_argument_count: usize,
    variant_no: usize,
    variadic_span: Span,
) -> syn::Result<ExpandedSqlFunction> {
    add_variadic_doc_comments(&mut input.attributes, &input.fn_name.to_string());

    let sql_name = parse_sql_name_attr(&mut input);

    input.fn_name = format_ident!("{}_{}", input.fn_name, variant_no);

    let nonvariadic_args_count = input
        .args
        .len()
        .checked_sub(variadic_argument_count)
        .ok_or_else(|| {
            syn::Error::new(
                variadic_span,
                "invalid variadic argument count: not enough function arguments",
            )
        })?;

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

    expand_nonvariadic(
        input,
        sql_name,
        legacy_helper_type_and_module,
        generate_return_type_helpers,
    )
}

fn add_variadic_doc_comments(
    attributes: &mut Vec<AttributeSpanWrapper<SqlFunctionAttribute>>,
    fn_name: &str,
) {
    let mut doc_comments_end = attributes.len()
        - attributes
            .iter()
            .rev()
            .position(|attr| matches!(&attr.item, SqlFunctionAttribute::Other(syn::Attribute{ meta: Meta::NameValue(MetaNameValue { path, .. }), ..}) if path.is_ident("doc")))
            .unwrap_or(attributes.len());

    let fn_family = format!("`{fn_name}_0`, `{fn_name}_1`, ... `{fn_name}_n`");

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
        ///
        /// For a greater convenience this environment variable can also be set
        /// in a `.cargo/config.toml` file as described in the
        /// [cargo documentation](https://doc.rust-lang.org/cargo/reference/config.html#env).
        #[doc(alias = #fn_name)]
    };

    for new_attribute in doc_comments {
        attributes.insert(
            doc_comments_end,
            AttributeSpanWrapper {
                item: SqlFunctionAttribute::Other(new_attribute),
                attribute_span: Span::mixed_site(),
                ident_span: Span::mixed_site(),
            },
        );
        doc_comments_end += 1;
    }
}

fn parse_sql_name_attr(input: &mut SqlFunctionDecl) -> String {
    let result = input
        .attributes
        .iter()
        .find_map(|attr| match attr.item {
            SqlFunctionAttribute::SqlName(_, ref value) => Some(value.value()),
            _ => None,
        })
        .unwrap_or_else(|| input.fn_name.to_string());

    result
}

fn expand_nonvariadic(
    input: SqlFunctionDecl,
    sql_name: String,
    legacy_helper_type_and_module: bool,
    generate_return_type_helpers: bool,
) -> syn::Result<ExpandedSqlFunction> {
    let SqlFunctionDecl {
        attributes,
        fn_token,
        fn_name,
        mut generics,
        args,
        return_type,
    } = input;

    let is_aggregate = attributes
        .iter()
        .any(|attr| matches!(attr.item, SqlFunctionAttribute::Aggregate(..)));

    let can_be_called_directly = !function_cannot_be_called_directly(&attributes);

    let skip_return_type_helper = attributes
        .iter()
        .any(|attr| matches!(attr.item, SqlFunctionAttribute::SkipReturnTypeHelper(..)));

    let window_attrs = attributes
        .iter()
        .filter(|a| matches!(a.item, SqlFunctionAttribute::Window { .. }))
        .cloned()
        .collect::<Vec<_>>();

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

    for StrictFnArg { name, .. } in &args {
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

    for window in &window_attrs {
        tokens.extend(generate_window_function_tokens(
            window,
            generics.clone(),
            &ty_generics,
            &fn_name,
        ));
    }
    if !window_attrs.is_empty() {
        tokens.extend(quote::quote! {
            impl #impl_generics IsWindowFunction for #fn_name #ty_generics {
                type ArgTypes = (#(#arg_name,)*);
            }
        });
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
            !window_attrs.is_empty(),
            &return_type,
            &sql_name,
        );
    } else if window_attrs.is_empty() {
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

    let (return_type_helper_module, return_type_helper_module_path) =
        if !generate_return_type_helpers || skip_return_type_helper {
            (None, None)
        } else {
            let auto_derived_types = type_args
                .iter()
                .map(|type_arg| {
                    for arg in &args {
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
                Ident::new(&format!("__{fn_name}_return_type"), fn_name.span());

            let doc =
                format!("Return type of the [`{fn_name}()`](fn@super::{fn_name}) SQL function.");
            let return_type_helper_module = quote! {
                #[allow(non_camel_case_types, non_snake_case, unused_imports)]
                #[doc(inline)]
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

fn generate_window_function_tokens(
    window: &AttributeSpanWrapper<SqlFunctionAttribute>,
    generics: Generics,
    ty_generics: &TypeGenerics<'_>,
    fn_name: &Ident,
) -> TokenStream {
    let SqlFunctionAttribute::Window {
        restrictions,
        require_order,
        ..
    } = &window.item
    else {
        unreachable!("We filtered for window attributes above")
    };
    restrictions.generate_all_window_fragment_impls(
        generics,
        ty_generics,
        fn_name,
        require_order.unwrap_or_default(),
    )
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
    is_window: bool,
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
    if is_supported_on_sqlite && !is_window {
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
) -> bool {
    let mut has_aggregate = false;
    let mut has_window = false;
    for attr in attributes {
        has_aggregate = has_aggregate || matches!(attr.item, SqlFunctionAttribute::Aggregate(..));
        has_window = has_window || matches!(attr.item, SqlFunctionAttribute::Window { .. });
    }
    has_window && !has_aggregate
}

pub(crate) struct ExternSqlBlock {
    pub(crate) function_decls: Vec<SqlFunctionDecl>,
}

impl Parse for ExternSqlBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut error = None::<syn::Error>;

        let mut combine_error = |e: syn::Error| {
            error = Some(
                error
                    .take()
                    .map(|mut o| {
                        o.combine(e.clone());
                        o
                    })
                    .unwrap_or(e),
            )
        };

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

        let parsed_block_attrs = parse_attributes(&mut combine_error, block.attrs);

        let item_count = block.items.len();
        let function_decls_input = block
            .items
            .into_iter()
            .map(|i| syn::parse2::<SqlFunctionDecl>(quote! { #i }));

        let mut function_decls = Vec::with_capacity(item_count);
        for decl in function_decls_input {
            match decl {
                Ok(mut decl) => {
                    decl.attributes = merge_attributes(&parsed_block_attrs, decl.attributes);
                    function_decls.push(decl)
                }
                Err(e) => {
                    error = Some(
                        error
                            .take()
                            .map(|mut o| {
                                o.combine(e.clone());
                                o
                            })
                            .unwrap_or(e),
                    );
                }
            }
        }

        error
            .map(Err)
            .unwrap_or(Ok(ExternSqlBlock { function_decls }))
    }
}

fn merge_attributes(
    parsed_block_attrs: &[AttributeSpanWrapper<SqlFunctionAttribute>],
    mut attributes: Vec<AttributeSpanWrapper<SqlFunctionAttribute>>,
) -> Vec<AttributeSpanWrapper<SqlFunctionAttribute>> {
    for attr in parsed_block_attrs {
        if attributes.iter().all(|a| match (&a.item, &attr.item) {
            (SqlFunctionAttribute::Aggregate(_), SqlFunctionAttribute::Aggregate(_)) => todo!(),
            (SqlFunctionAttribute::Window { .. }, SqlFunctionAttribute::Window { .. })
            | (SqlFunctionAttribute::SqlName(_, _), SqlFunctionAttribute::SqlName(_, _))
            | (SqlFunctionAttribute::Restriction(_), SqlFunctionAttribute::Restriction(_))
            | (SqlFunctionAttribute::Variadic(_, _), SqlFunctionAttribute::Variadic(_, _))
            | (
                SqlFunctionAttribute::SkipReturnTypeHelper(_),
                SqlFunctionAttribute::SkipReturnTypeHelper(_),
            ) => false,
            _ => true,
        }) {
            attributes.push(attr.clone());
        }
    }
    attributes
}

#[derive(Clone)]
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
        let mut error = None::<syn::Error>;
        let mut combine_error = |e: syn::Error| {
            error = Some(
                error
                    .take()
                    .map(|mut o| {
                        o.combine(e.clone());
                        o
                    })
                    .unwrap_or(e),
            )
        };

        let attributes = Attribute::parse_outer(input).unwrap_or_else(|e| {
            combine_error(e);
            Vec::new()
        });
        let attributes_collected = parse_attributes(&mut combine_error, attributes);

        let fn_token: Token![fn] = input.parse().unwrap_or_else(|e| {
            combine_error(e);
            Default::default()
        });
        let fn_name = Ident::parse(input).unwrap_or_else(|e| {
            combine_error(e);
            Ident::new("dummy", Span::call_site())
        });
        let generics = Generics::parse(input).unwrap_or_else(|e| {
            combine_error(e);
            Generics {
                lt_token: None,
                params: Punctuated::new(),
                gt_token: None,
                where_clause: None,
            }
        });
        let args;
        let _paren = parenthesized!(args in input);
        let args = args
            .parse_terminated(StrictFnArg::parse, Token![,])
            .unwrap_or_else(|e| {
                combine_error(e);
                Punctuated::new()
            });
        let rarrow = Option::<Token![->]>::parse(input).unwrap_or_else(|e| {
            combine_error(e);
            None
        });
        let return_type = if rarrow.is_some() {
            Type::parse(input).unwrap_or_else(|e| {
                combine_error(e);
                Type::Never(syn::TypeNever {
                    bang_token: Default::default(),
                })
            })
        } else {
            parse_quote!(diesel::expression::expression_types::NotSelectable)
        };
        let _semi = Option::<Token![;]>::parse(input).unwrap_or_else(|e| {
            combine_error(e);
            None
        });

        error.map(Err).unwrap_or(Ok(Self {
            attributes: attributes_collected,
            fn_token,
            fn_name,
            generics,
            args,
            return_type,
        }))
    }
}

fn parse_attribute(
    attr: syn::Attribute,
) -> Result<Option<AttributeSpanWrapper<SqlFunctionAttribute>>> {
    match &attr.meta {
        syn::Meta::NameValue(syn::MetaNameValue {
            path,
            value:
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(sql_name),
                    ..
                }),
            ..
        }) if path.is_ident("sql_name") => Ok(Some(AttributeSpanWrapper {
            attribute_span: attr.span(),
            ident_span: sql_name.span(),
            item: SqlFunctionAttribute::SqlName(path.require_ident()?.clone(), sql_name.clone()),
        })),
        syn::Meta::Path(path) if path.is_ident("aggregate") => Ok(Some(AttributeSpanWrapper {
            attribute_span: attr.span(),
            ident_span: path.span(),
            item: SqlFunctionAttribute::Aggregate(
                path.require_ident()
                    .map_err(|e| {
                        syn::Error::new(
                            e.span(),
                            format!("{e}, the correct format is `#[aggregate]`"),
                        )
                    })?
                    .clone(),
            ),
        })),
        syn::Meta::Path(path) if path.is_ident("skip_return_type_helper") => {
            Ok(Some(AttributeSpanWrapper {
                ident_span: attr.span(),
                attribute_span: path.span(),
                item: SqlFunctionAttribute::SkipReturnTypeHelper(
                    path.require_ident()
                        .map_err(|e| {
                            syn::Error::new(
                                e.span(),
                                format!("{e}, the correct format is `#[skip_return_type_helper]`"),
                            )
                        })?
                        .clone(),
                ),
            }))
        }
        syn::Meta::Path(path) if path.is_ident("window") => Ok(Some(AttributeSpanWrapper {
            attribute_span: attr.span(),
            ident_span: path.span(),
            item: SqlFunctionAttribute::Window {
                ident: path
                    .require_ident()
                    .map_err(|e| {
                        syn::Error::new(e.span(), format!("{e}, the correct format is `#[window]`"))
                    })?
                    .clone(),
                restrictions: BackendRestriction::None,
                require_order: None,
            },
        })),
        syn::Meta::List(syn::MetaList {
            path,
            delimiter: syn::MacroDelimiter::Paren(_),
            tokens,
        }) if path.is_ident("variadic") => {
            let count: syn::LitInt = syn::parse2(tokens.clone()).map_err(|e| {
                syn::Error::new(
                    e.span(),
                    format!("{e}, the correct format is `#[variadic(3)]`"),
                )
            })?;
            Ok(Some(AttributeSpanWrapper {
                item: SqlFunctionAttribute::Variadic(
                    path.require_ident()
                        .map_err(|e| {
                            syn::Error::new(
                                e.span(),
                                format!("{e}, the correct format is `#[variadic(3)]`"),
                            )
                        })?
                        .clone(),
                    count.clone(),
                ),
                attribute_span: attr.span(),
                ident_span: path.require_ident()?.span(),
            }))
        }
        syn::Meta::NameValue(_) | syn::Meta::Path(_) => Ok(Some(AttributeSpanWrapper {
            attribute_span: attr.span(),
            ident_span: attr.span(),
            item: SqlFunctionAttribute::Other(attr),
        })),
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
    }
}

fn parse_attributes(
    combine_error: &mut impl FnMut(syn::Error),
    attributes: Vec<Attribute>,
) -> Vec<AttributeSpanWrapper<SqlFunctionAttribute>> {
    let attribute_count = attributes.len();

    let attributes = attributes
        .into_iter()
        .filter_map(|attr| parse_attribute(attr).transpose());

    let mut attributes_collected = Vec::with_capacity(attribute_count);
    for attr in attributes {
        match attr {
            Ok(attr) => attributes_collected.push(attr),
            Err(e) => {
                combine_error(e);
            }
        }
    }
    attributes_collected
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
        require_order: bool,
    ) -> TokenStream {
        generics.params.push(parse_quote!(__P));
        generics.params.push(parse_quote!(__O));
        generics.params.push(parse_quote!(__F));
        let order = if require_order {
            quote::quote! {
                diesel::internal::sql_functions::Order<__O, true>
            }
        } else {
            quote::quote! {__O}
        };
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
                    &order,
                )
            }
            BackendRestriction::SqlDialect(_, ref dialect, ref dialect_type) => {
                generics.params.push(parse_quote!(__DieselInternal));
                let (impl_generics, _, _) = generics.split_for_impl();
                let mut out = quote::quote! {
                    impl #impl_generics WindowFunctionFragment<#fn_name #ty_generics, __DieselInternal>
                        for OverClause<__P, #order, __F>
                    where
                        Self: WindowFunctionFragment<#fn_name #ty_generics, __DieselInternal, <__DieselInternal as diesel::backend::SqlDialect>::#dialect>,
                        __DieselInternal: diesel::backend::Backend,
                    {
                    }

                };
                let specific_impl = Self::generate_window_fragment_impl(
                    parse_quote!(__DieselInternal),
                    Some(
                        parse_quote!(__DieselInternal: diesel::backend::Backend + diesel::backend::SqlDialect<#dialect = #dialect_type>,),
                    ),
                    &impl_generics,
                    ty_generics,
                    fn_name,
                    Some(dialect_type),
                    &order,
                );
                out.extend(specific_impl);
                out
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
                    &order,
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
                        &order,
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
        order: &TokenStream,
    ) -> TokenStream {
        quote::quote! {
            impl #impl_generics WindowFunctionFragment<#fn_name #ty_generics, #backend, #dialect> for OverClause<__P, #order, __F>
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
    Window {
        ident: Ident,
        restrictions: BackendRestriction,
        require_order: Option<bool>,
    },
    SqlName(Ident, LitStr),
    Restriction(BackendRestriction),
    Variadic(Ident, LitInt),
    SkipReturnTypeHelper(Ident),
    Other(Attribute),
}

impl MySpanned for SqlFunctionAttribute {
    fn span(&self) -> proc_macro2::Span {
        match self {
            SqlFunctionAttribute::Restriction(BackendRestriction::Backends(ref ident, ..))
            | SqlFunctionAttribute::Restriction(BackendRestriction::SqlDialect(ref ident, ..))
            | SqlFunctionAttribute::Restriction(BackendRestriction::BackendBound(ref ident, ..))
            | SqlFunctionAttribute::Aggregate(ref ident, ..)
            | SqlFunctionAttribute::Window { ref ident, .. }
            | SqlFunctionAttribute::Variadic(ref ident, ..)
            | SqlFunctionAttribute::SkipReturnTypeHelper(ref ident)
            | SqlFunctionAttribute::SqlName(ref ident, ..) => ident.span(),
            SqlFunctionAttribute::Restriction(BackendRestriction::None) => {
                unreachable!("We do not construct that")
            }
            SqlFunctionAttribute::Other(ref attribute) => attribute.span(),
        }
    }
}

fn parse_require_order(input: &syn::parse::ParseBuffer<'_>) -> Result<bool> {
    let ident = input.parse::<Ident>()?;
    if ident == "require_order" {
        let _ = input.parse::<Token![=]>()?;
        let value = input.parse::<LitBool>()?;
        Ok(value.value)
    } else {
        Err(syn::Error::new(
            ident.span(),
            format!("Expected `require_order` but got `{ident}`"),
        ))
    }
}

impl SqlFunctionAttribute {
    fn parse_attr(
        name: Ident,
        input: &syn::parse::ParseBuffer<'_>,
        attr: Attribute,
        attribute_span: proc_macro2::Span,
    ) -> Result<Option<AttributeSpanWrapper<Self>>> {
        // rustc doesn't resolve cfg attrs for us :(
        // This is hacky, but mostly for internal use
        if name == "cfg_attr" {
            let ident = input.parse::<Ident>()?;
            if ident != "feature" {
                return Err(syn::Error::new(
                    ident.span(),
                    format!(
                        "only single feature `cfg_attr` attributes are supported. \
                             Got `{ident}` but expected `feature = \"foo\"`"
                    ),
                ));
            }
            let _ = input.parse::<Token![=]>()?;
            let feature = input.parse::<LitStr>()?;
            let feature_value = feature.value();
            let _ = input.parse::<Token![,]>()?;
            let ignore = match feature_value.as_str() {
                "postgres_backend" => !cfg!(feature = "postgres"),
                "sqlite" => !cfg!(feature = "sqlite"),
                "mysql_backend" => !cfg!(feature = "mysql"),
                feature => {
                    return Err(syn::Error::new(
                        feature.span(),
                        format!(
                            "only `mysql_backend`, `postgres_backend` and `sqlite` \
                                 are supported features, but got `{feature}`"
                        ),
                    ));
                }
            };
            let name = input.parse::<Ident>()?;
            let inner;
            let _paren = parenthesized!(inner in input);
            let ret = SqlFunctionAttribute::parse_attr(name, &inner, attr, attribute_span)?;
            if ignore {
                Ok(None)
            } else {
                Ok(ret)
            }
        } else {
            let name_str = name.to_string();
            let parsed_attr = match &*name_str {
                "window" => {
                    let restrictions = if BackendRestriction::parse_from(&input.fork()).is_ok() {
                        BackendRestriction::parse_from(input).map(Ok).ok()
                    } else {
                        None
                    };
                    if input.fork().parse::<Token![,]>().is_ok() {
                        let _ = input.parse::<Token![,]>()?;
                    }
                    let require_order = if parse_require_order(&input.fork()).is_ok() {
                        Some(parse_require_order(input)?)
                    } else {
                        None
                    };
                    if input.fork().parse::<Token![,]>().is_ok() {
                        let _ = input.parse::<Token![,]>()?;
                    }
                    let restrictions =
                        restrictions.unwrap_or_else(|| BackendRestriction::parse_from(input))?;
                    Self::Window {
                        ident: name,
                        restrictions,
                        require_order,
                    }
                }
                "sql_name" => {
                    parse_eq(input, "sql_name = \"SUM\"").map(|v| Self::SqlName(name, v))?
                }
                "backends" => {
                    BackendRestriction::parse_backends(input, name).map(Self::Restriction)?
                }
                "dialect" => {
                    BackendRestriction::parse_sql_dialect(input, name).map(Self::Restriction)?
                }
                "backend_bounds" => {
                    BackendRestriction::parse_backend_bounds(input, name).map(Self::Restriction)?
                }
                "variadic" => Self::Variadic(name, input.parse()?),
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
            Ok(Some(AttributeSpanWrapper {
                ident_span: parsed_attr.span(),
                item: parsed_attr,
                attribute_span,
            }))
        }
    }
}

#[derive(Default)]
pub(crate) struct DeclareSqlFunctionArgs {
    pub(crate) generate_return_type_helpers: bool,
}

impl DeclareSqlFunctionArgs {
    pub(crate) fn parse_from_macro_input(input: TokenStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self::default());
        }
        let input_span = input.span();
        let parsed: syn::MetaNameValue = syn::parse2(input).map_err(|e| {
            let span = e.span();
            syn::Error::new(
                span,
                format!("{e}, the correct format is `generate_return_type_helpers = true/false`"),
            )
        })?;
        match parsed {
            syn::MetaNameValue {
                path,
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Bool(b),
                        ..
                    }),
                ..
            } if path.is_ident("generate_return_type_helpers") => Ok(Self {
                generate_return_type_helpers: b.value,
            }),
            _ => Err(syn::Error::new(input_span, "Invalid config")),
        }
    }
}
