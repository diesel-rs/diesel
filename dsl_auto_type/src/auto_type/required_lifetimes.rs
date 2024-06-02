use {
    std::rc::Rc,
    syn::visit::{self, Visit},
};

pub(crate) fn extract_referenced_lifetimes<'t>(
    ty: &'t syn::Type,
    errors: &mut Vec<Rc<syn::Error>>,
) -> Vec<&'t syn::Lifetime> {
    struct LifetimeVisitor<'t, 'errs> {
        lifetimes: Vec<&'t syn::Lifetime>,
        errors: &'errs mut Vec<Rc<syn::Error>>,
    }

    impl<'ast> Visit<'ast> for LifetimeVisitor<'ast, '_> {
        fn visit_lifetime(&mut self, lifetime: &'ast syn::Lifetime) {
            if lifetime.ident == "_" {
                self.errors.push(Rc::new(syn::Error::new_spanned(
                    lifetime,
                    "`#[auto_type]` requires named lifetimes",
                )));
            }
            if lifetime.ident != "static" {
                self.lifetimes.push(lifetime);
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
    }

    let mut visitor = LifetimeVisitor {
        lifetimes: Vec::new(),
        errors,
    };

    visitor.visit_type(ty);

    visitor.lifetimes
}
