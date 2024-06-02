use super::*;

/// The map itself, + some settings to run the inferrer with
pub(crate) struct LocalVariablesMap<'a, 'p> {
    pub(crate) inferrer_settings: &'a InferrerSettings,
    /// The map, with an optional parent (to support nested blocks)
    pub(crate) inner: LocalVariablesMapInner<'a, 'p>,
}
/// The map, with an optional parent (to support nested blocks)
pub(crate) struct LocalVariablesMapInner<'a, 'p> {
    pub(crate) map: HashMap<&'a Ident, LetStatementInferredType>,
    pub(crate) parent: Option<&'p LocalVariablesMapInner<'a, 'p>>,
}
pub(crate) struct LetStatementInferredType {
    pub(crate) type_: Type,
    pub(crate) errors: Vec<Rc<syn::Error>>,
}
impl<'a, 'p> LocalVariablesMapInner<'a, 'p> {
    /// Lookup in this map, and if not found, in the parent map
    /// This is to support nested blocks
    pub(crate) fn get(&self, ident: &Ident) -> Option<&LetStatementInferredType> {
        match self.map.get(ident) {
            Some(inferred_type) => Some(inferred_type),
            None => match self.parent {
                Some(parent) => parent.get(ident),
                None => None,
            },
        }
    }
}

impl<'a, 'p> LocalVariablesMap<'a, 'p> {
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
                self.inner.map.insert(
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

    pub(crate) fn process_const_generic(&mut self, const_generic: &'a syn::ConstParam) {
        self.inner.map.insert(
            &const_generic.ident,
            LetStatementInferredType {
                type_: const_generic.ty.clone(),
                errors: Vec::new(),
            },
        );
    }

    /// Finishes a block inference for this map.
    /// It may be initialized with `pat`s before (such as function parameters),
    /// then this function is used to infer the type of the last expression in the block.
    ///
    /// It takes the last expression of the block as a `block_last_expr`
    /// parameter, because depending on where this is called,
    /// `return`/`let`,`function_call();` will or not be tolerated, so this is
    /// matched before calling this function.
    ///
    /// Expects that the block has at least one statement (it is assumed that
    /// the last statement is provided as `block_last_expr`)
    pub(crate) fn infer_block_expression_type(
        mut self,
        block_last_expr: &'a syn::Expr,
        block_type_hint: Option<&'a syn::Type>,
        block: &'a syn::Block,
        errors: &mut Vec<Rc<syn::Error>>,
    ) -> syn::Type {
        for statement in &block.stmts[0..block
            .stmts
            .len()
            .checked_sub(1)
            .expect("Block should have at least one statement, provided as `block_last_expr`")]
        {
            if let syn::Stmt::Local(local) = statement {
                match self.process_pat(
                    &local.pat,
                    None,
                    local.init.as_ref().map(|local_init| &*local_init.expr),
                ) {
                    Ok(()) => {}
                    Err(e) => {
                        errors.push(Rc::new(e));
                    }
                }
            };
        }

        if errors.is_empty() {
            // We haven't encountered any error so the local variables map is
            // properly initialized.
            // If there are no such "syntax errors", we may attempt parsing.
            let inferrer = self.inferrer();
            let block_output_type =
                inferrer.infer_expression_type(block_last_expr, block_type_hint);
            errors.extend(inferrer.into_errors());
            block_output_type
        } else {
            // We don't attempt inference if there are already errors because `process_pat`
            // is already pretty tolerant, and we don't want to only show these errors
            // once another error starts happening, as they may be confusing to
            // the user.
            let block_span = block.span();
            parse_quote_spanned!(block_span=> _)
        }
    }
}
