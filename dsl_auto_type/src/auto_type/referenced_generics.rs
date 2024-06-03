use std::rc::Rc;
use syn::parse_quote;
use syn::visit::{self, Visit};
use syn::{Ident, Lifetime};

pub(crate) fn extract_referenced_generics(
    ty: &syn::Type,
    generics: &syn::Generics,
    errors: &mut Vec<Rc<syn::Error>>,
) -> syn::Generics {
    struct Visitor<'g, 'errs> {
        lifetimes: Vec<(&'g Lifetime, bool)>,
        type_parameters: Vec<(&'g Ident, bool)>,
        errors: &'errs mut Vec<Rc<syn::Error>>,
    }

    let mut visitor = Visitor {
        lifetimes: generics
            .lifetimes()
            .map(|lt| (&lt.lifetime, false))
            .collect(),
        type_parameters: generics
            .type_params()
            .map(|tp| (&tp.ident, false))
            .collect(),
        errors,
    };
    visitor.lifetimes.sort_unstable();
    visitor.type_parameters.sort_unstable();

    impl<'ast> Visit<'ast> for Visitor<'_, '_> {
        fn visit_lifetime(&mut self, lifetime: &'ast Lifetime) {
            if lifetime.ident == "_" {
                self.errors.push(Rc::new(syn::Error::new_spanned(
                    lifetime,
                    "`#[auto_type]` requires named lifetimes",
                )));
            } else if lifetime.ident != "static" {
                if let Ok(lifetime_idx) = self
                    .lifetimes
                    .binary_search_by_key(&lifetime, |(lt, _)| *lt)
                {
                    self.lifetimes[lifetime_idx].1 = true;
                }
            }
            visit::visit_lifetime(self, lifetime)
        }

        fn visit_type_reference(&mut self, reference: &'ast syn::TypeReference) {
            if reference.lifetime.is_none() {
                self.errors.push(Rc::new(syn::Error::new_spanned(
                    reference,
                    "`#[auto_type]` requires named lifetimes",
                )));
            }
            visit::visit_type_reference(self, reference)
        }

        fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
            if let Some(path_ident) = type_path.path.get_ident() {
                if let Ok(type_param_idx) = self
                    .type_parameters
                    .binary_search_by_key(&path_ident, |tp| tp.0)
                {
                    self.type_parameters[type_param_idx].1 = true;
                }
            }
            visit::visit_type_path(self, type_path)
        }
    }

    visitor.visit_type(ty);

    let generic_params: syn::punctuated::Punctuated<syn::GenericParam, _> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Lifetime(lt)
                if visitor
                    .lifetimes
                    .binary_search(&(&lt.lifetime, true))
                    .is_ok() =>
            {
                let lt = &lt.lifetime;
                Some(parse_quote!(#lt))
            }
            syn::GenericParam::Type(tp)
                if visitor
                    .type_parameters
                    .binary_search(&(&tp.ident, true))
                    .is_ok() =>
            {
                let ident = &tp.ident;
                Some(parse_quote!(#ident))
            }
            _ => None::<syn::GenericParam>,
        })
        .collect();

    // We need to not set the lt_token and gt_token if `params` is empty to get
    // a reasonable error message for the case that there is no lifetime specifier
    // but we need one
    syn::Generics {
        lt_token: (!generic_params.is_empty()).then(Default::default),
        gt_token: (!generic_params.is_empty()).then(Default::default),
        params: generic_params,
        where_clause: None,
    }
}
