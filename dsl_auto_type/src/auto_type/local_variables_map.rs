use super::*;

pub(crate) struct LocalVariablesMap<'a> {
    pub(crate) inferrer_settings: &'a InferrerSettings,
    pub(crate) map: HashMap<&'a Ident, LetStatementInferredType>,
}
pub(crate) struct LetStatementInferredType {
    pub(crate) type_: Type,
    pub(crate) errors: Vec<Rc<syn::Error>>,
}

impl<'a> LocalVariablesMap<'a> {
    pub(crate) fn process_pat(
        &mut self,
        pat: &'a syn::Pat,
        type_ascription: Option<&'a Type>,
        local_init_expression: Option<&'a syn::Expr>,
    ) -> Result<(), syn::Error> {
        // Either the let statement hints the type or we have to infer it
        // Either the let statement is a simple assignment or a destructuring assignment
        match pat {
            syn::Pat::Type(pat_type) => {
                self.process_pat(
                    &pat_type.pat,
                    Some(match type_ascription {
                        None => &pat_type.ty,
                        Some(type_ascription) => {
                            return Err(syn::Error::new(
                                type_ascription.span(),
                                "auto_type: unexpected double type ascription",
                            ))
                        }
                    }),
                    local_init_expression,
                )?;
            }
            syn::Pat::Ident(pat_ident) => {
                self.map.insert(
                    &pat_ident.ident,
                    match (type_ascription, local_init_expression) {
                        (opt_type_ascription, Some(expr)) => {
                            let inferrer = self.inferrer();
                            LetStatementInferredType {
                                type_: inferrer.infer_expression_type(expr, opt_type_ascription),
                                errors: inferrer.into_errors(),
                            }
                        }
                        (Some(type_ascription), None) => LetStatementInferredType {
                            type_: type_ascription.clone(),
                            errors: Vec::new(),
                        },
                        (None, None) => LetStatementInferredType {
                            type_: parse_quote_spanned!(pat_ident.span()=> _),
                            errors: vec![Rc::new(syn::Error::new_spanned(
                                pat_ident,
                                "auto_type: Let statement with no type ascription \
                                    and no initializer expression is not supported",
                            ))],
                        },
                    },
                );
            }
            syn::Pat::Tuple(pat_tuple) => {
                if let Some(type_ascription) = type_ascription {
                    if let Type::Tuple(type_tuple) = type_ascription {
                        if pat_tuple.elems.len() != type_tuple.elems.len() {
                            return Err(syn::Error::new(
                                type_ascription.span(),
                                "auto_type: tuple let assignment and its \
                                    type ascription have different number of elements",
                            ));
                        }
                    }
                }
                for (i, pat) in pat_tuple.elems.iter().enumerate() {
                    self.process_pat(
                        pat,
                        match type_ascription {
                            Some(Type::Tuple(type_tuple)) => Some(&type_tuple.elems[i]),
                            _ => None,
                        },
                        match local_init_expression {
                            Some(syn::Expr::Tuple(expr_tuple)) => Some(&expr_tuple.elems[i]),
                            _ => None,
                        },
                    )?;
                }
            }
            _ => {
                // We won't be able to infer these, at the same time we don't
                // want to error because there may be valid user
                // code using these, and we won't need it if these variables
                // are not needed to infer the type of the final expression.
            }
        };
        Ok(())
    }
}
