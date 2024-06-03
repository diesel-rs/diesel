use super::*;

pub use super::settings_builder::InferrerSettingsBuilder;

/// This is meant to be used if there's need to infer a single expression type
/// out of the context of a function. It will be assumed that there are no
/// intermediate variables (`let` statements). initially, but one can still use
/// intermediate block expression to annotate types.
///
/// This is useful in the context of Diesel's `Selectable` macro.
pub fn infer_expression_type(
    expr: &syn::Expr,
    type_hint: Option<&syn::Type>,
    inferrer_settings: &InferrerSettings,
) -> (syn::Type, Vec<syn::Error>) {
    let local_variables_map = LocalVariablesMap {
        inferrer_settings,
        inner: LocalVariablesMapInner {
            map: Default::default(),
            parent: None,
        },
    };
    let inferrer = local_variables_map.inferrer();
    let type_ = inferrer.infer_expression_type(expr, type_hint);

    let errors = inferrer
        .into_errors()
        .into_iter()
        .filter_map(|rc| {
            // Extracting from the `Rc` only if it's the last reference is an elegant way to
            // deduplicate errors For this to work it is necessary that the rest of
            // the errors (those from the local variables map that weren't used) are
            // dropped before, which is the case here, and that we are iterating on the
            // errors in an owned manner.
            Rc::try_unwrap(rc).ok()
        })
        .collect();
    (type_, errors)
}

pub struct InferrerSettings {
    pub(crate) dsl_path: syn::Path,
    pub(crate) method_types_case: Case,
    pub(crate) function_types_case: Case,
}

impl<'a, 'p> LocalVariablesMap<'a, 'p> {
    pub(crate) fn inferrer(&'a self) -> TypeInferrer<'a> {
        TypeInferrer {
            local_variables_map: self,
            errors: Default::default(),
        }
    }
}

pub(crate) struct TypeInferrer<'s> {
    local_variables_map: &'s LocalVariablesMap<'s, 's>,
    errors: std::cell::RefCell<Vec<Rc<syn::Error>>>,
}

impl TypeInferrer<'_> {
    /// Calls `try_infer_expression_type` and falls back to `_` if it fails,
    /// collecting the error for display
    pub(crate) fn infer_expression_type(
        &self,
        expr: &syn::Expr,
        type_hint: Option<&syn::Type>,
    ) -> syn::Type {
        let inferred = self.try_infer_expression_type(expr, type_hint);

        match inferred {
            Ok(t) => t,
            Err(e) => self.register_error(e, expr.span()),
        }
    }

    fn register_error(&self, error: syn::Error, infer_type_span: Span) -> syn::Type {
        self.errors.borrow_mut().push(Rc::new(error));
        parse_quote_spanned!(infer_type_span=> _)
    }

    fn try_infer_expression_type(
        &self,
        expr: &syn::Expr,
        type_hint: Option<&syn::Type>,
    ) -> Result<syn::Type, syn::Error> {
        let expression_type: syn::Type = match (
            expr,
            type_hint.filter(|h| !matches!(h, syn::Type::Infer(_))),
        ) {
            (syn::Expr::Group(syn::ExprGroup { expr, .. }), type_hint) => {
                return self.try_infer_expression_type(expr, type_hint)
            }
            (
                syn::Expr::Tuple(syn::ExprTuple {
                    elems: expr_elems, ..
                }),
                Some(syn::Type::Tuple(
                    type_tuple @ syn::TypeTuple {
                        elems: type_elems, ..
                    },
                )),
            ) => {
                if type_elems.len() != expr_elems.len() {
                    return Err(syn::Error::new(
                        type_tuple.span(),
                        "auto_type: tuple type and its \
                            expression have different number of elements",
                    ));
                }
                syn::Type::Tuple(syn::TypeTuple {
                    elems: type_elems
                        .iter()
                        .zip(expr_elems.iter())
                        .map(|(type_, expr)| self.infer_expression_type(expr, Some(type_)))
                        .collect(),
                    ..type_tuple.clone()
                })
            }
            (syn::Expr::Tuple(syn::ExprTuple { elems, .. }), None) => {
                syn::Type::Tuple(syn::TypeTuple {
                    elems: elems
                        .iter()
                        .map(|e| self.infer_expression_type(e, None))
                        .collect(),
                    paren_token: Default::default(),
                })
            }
            (syn::Expr::Path(syn::ExprPath { path, .. }), None) => {
                // This is either a local variable or we should assume that the type exists at
                // the same path as the function (with applied casing for last segment)
                let path_is_ident = path.get_ident();
                if path_is_ident.map_or(false, |ident| ident == "self") {
                    parse_quote!(Self)
                } else if let Some(LetStatementInferredType { type_, errors }) = path_is_ident
                    .and_then(|path_single_ident| {
                        self.local_variables_map.inner.get(path_single_ident)
                    })
                {
                    // Since we are using this type for the computation of the current type, any
                    // errors encountered there are relevant here
                    self.errors.borrow_mut().extend(errors.iter().cloned());
                    type_.clone()
                } else {
                    syn::Type::Path(syn::TypePath {
                        path: path.clone(),
                        qself: None,
                    })
                }
            }
            (syn::Expr::Call(syn::ExprCall { func, args, .. }), None) => {
                let unsupported_function_type = || {
                    syn::Error::new_spanned(
                        &**func,
                        "unsupported function type for auto_type, please provide a type hint",
                    )
                };
                let func_type = self.try_infer_expression_type(func, None)?;
                // First we extract the type of the function
                let mut type_path = match func_type {
                    syn::Type::Path(syn::TypePath { path, .. }) => path,
                    _ => return Err(unsupported_function_type()),
                };
                // Then we update the case if specified
                if self
                    .local_variables_map
                    .inferrer_settings
                    .function_types_case
                    != Case::DoNotChange
                {
                    if let Some(last) = type_path.segments.last_mut() {
                        last.ident = self
                            .local_variables_map
                            .inferrer_settings
                            .function_types_case
                            .ident_with_case(&last.ident);
                    }
                }
                // Then we will add the generic arguments
                let last_segment = type_path
                    .segments
                    .last_mut()
                    .ok_or_else(unsupported_function_type)?;
                last_segment.arguments = self.infer_generics_or_use_hints(
                    None,
                    args,
                    match &last_segment.arguments {
                        syn::PathArguments::None => None,
                        syn::PathArguments::AngleBracketed(ab) => Some(ab),
                        syn::PathArguments::Parenthesized(_) => {
                            return Err(unsupported_function_type())
                        }
                    },
                )?;
                syn::Type::Path(syn::TypePath {
                    path: type_path,
                    qself: None,
                })
            }
            (
                syn::Expr::MethodCall(syn::ExprMethodCall {
                    receiver,
                    method,
                    turbofish,
                    args,
                    ..
                }),
                None,
            ) => syn::Type::Path(syn::TypePath {
                path: syn::Path {
                    segments: self
                        .local_variables_map
                        .inferrer_settings
                        .dsl_path
                        .segments
                        .iter()
                        .cloned()
                        .chain([syn::PathSegment {
                            ident: self
                                .local_variables_map
                                .inferrer_settings
                                .method_types_case
                                .ident_with_case(method),
                            arguments: self.infer_generics_or_use_hints(
                                Some(syn::GenericArgument::Type(
                                    self.infer_expression_type(receiver, None),
                                )),
                                args,
                                turbofish.as_ref(),
                            )?,
                        }])
                        .collect(),
                    leading_colon: None,
                },
                qself: None,
            }),
            (syn::Expr::Lit(syn::ExprLit { lit, .. }), None) => match lit {
                syn::Lit::Str(_) => parse_quote_spanned!(lit.span()=> &'static str),
                syn::Lit::ByteStr(_) => parse_quote_spanned!(lit.span()=> &'static [u8]),
                syn::Lit::Byte(_) => parse_quote_spanned!(lit.span()=> u8),
                syn::Lit::Char(_) => parse_quote_spanned!(lit.span()=> char),
                syn::Lit::Int(lit_int) => literal_type(&lit_int.token())?,
                syn::Lit::Float(lit_float) => literal_type(&lit_float.token())?,
                syn::Lit::Bool(_) => parse_quote_spanned!(lit.span()=> bool),
                _ => {
                    return Err(syn::Error::new(
                        lit.span(),
                        "unsupported literal for auto_type, please provide a type hint",
                    ))
                }
            },
            (syn::Expr::Block(syn::ExprBlock { block, .. }), type_hint) => {
                match block.stmts.last() {
                    None
                    | Some(
                        syn::Stmt::Local(_) | syn::Stmt::Item(_) | syn::Stmt::Expr(_, Some(_)),
                    ) => {
                        // Empty blocks, local variables (`let`) and other item definition as last
                        // statement, as well as expressions BUT with a semicolon, lead to the
                        // block having unit type.
                        match type_hint {
                            Some(type_hint) => {
                                // Prefer user-specified type hint to our own inference
                                type_hint.clone()
                            }
                            None => parse_quote_spanned!(block.span()=> ()),
                        }
                    }
                    Some(syn::Stmt::Expr(expr, None)) => {
                        let local_variables_map = LocalVariablesMap {
                            inferrer_settings: self.local_variables_map.inferrer_settings,
                            inner: LocalVariablesMapInner {
                                map: Default::default(),
                                parent: Some(&self.local_variables_map.inner),
                            },
                        };
                        local_variables_map.infer_block_expression_type(
                            expr,
                            type_hint,
                            block,
                            &mut self.errors.borrow_mut(),
                        )
                    }
                    Some(syn::Stmt::Macro(syn::StmtMacro { mac, .. })) => {
                        match type_hint {
                            Some(type_hint) => {
                                // User provided a type hint to the macro expression, we won't be
                                // able to do any better than this
                                type_hint.clone()
                            }
                            None => {
                                return Err(syn::Error::new_spanned(
                                    mac,
                                    "auto_type: unsupported macro call as last statement in block, \
                                        please provide a type hint",
                                ));
                            }
                        }
                    }
                }
            }
            (_, None) => {
                return Err(syn::Error::new(
                    expr.span(),
                    "unsupported expression for auto_type, please provide a type hint",
                ))
            }
            (_, Some(type_hint)) => type_hint.clone(),
        };
        Ok(expression_type)
    }

    /// `infer` is always supposed to be a syn::Type::Infer
    fn infer_generics_or_use_hints(
        &self,
        add_first: Option<syn::GenericArgument>,
        args: &syn::punctuated::Punctuated<syn::Expr, Token![,]>,
        hint: Option<&syn::AngleBracketedGenericArguments>,
    ) -> Result<syn::PathArguments, syn::Error> {
        let arguments = syn::AngleBracketedGenericArguments {
            args: add_first
                .into_iter()
                .chain(match hint {
                    None => {
                        // We should infer
                        Either::Left(args.iter().map(|e| {
                            syn::GenericArgument::Type(self.infer_expression_type(e, None))
                        }))
                    }
                    Some(hint_or_override) => Either::Right(
                        hint_or_override
                            .args
                            .iter()
                            .zip(args.iter().map(Some).chain((0..).map(|_| None)))
                            .map(|(hint, expr)| match (hint, expr) {
                                (syn::GenericArgument::Type(type_hint), Some(expr)) => {
                                    syn::GenericArgument::Type(
                                        self.infer_expression_type(expr, Some(type_hint)),
                                    )
                                }
                                (
                                    generic_argument @ syn::GenericArgument::Type(syn::Type::Infer(
                                        _,
                                    )),
                                    None,
                                ) => syn::GenericArgument::Type(self.register_error(
                                    syn::Error::new_spanned(
                                        generic_argument,
                                        "auto_type: Can't infer generic argument because \
                                            there is no function argument to infer from \
                                            (less function arguments than generic arguments)",
                                    ),
                                    generic_argument.span(),
                                )),
                                (generic_argument, _) => generic_argument.clone(),
                            }),
                    ),
                })
                .collect(),
            colon2_token: None, // there is no colon2 in types, only in function calls
            lt_token: Default::default(),
            gt_token: Default::default(),
        };
        Ok(if arguments.args.is_empty() {
            syn::PathArguments::None
        } else {
            syn::PathArguments::AngleBracketed(arguments)
        })
    }

    pub(crate) fn into_errors(self) -> Vec<Rc<syn::Error>> {
        self.errors.into_inner()
    }
}

fn literal_type(t: &proc_macro2::Literal) -> Result<syn::Type, syn::Error> {
    let val = t.to_string();
    let type_suffix = &val[val
        .find(|c: char| !c.is_ascii_digit() && c != '_')
        .ok_or_else(|| {
            syn::Error::new_spanned(
                t,
                format_args!("Literals must have type suffix for auto_type, e.g. {val}i64"),
            )
        })?..];
    syn::parse_str(type_suffix)
        .map_err(|_| syn::Error::new_spanned(t, "Invalid type suffix for literal"))
}
