use syntax::ast::TyKind;
use syntax::ast;
use syntax::attr::AttrMetaMethods;
use syntax::codemap::Span;
use syntax::ext::base::ExtCtxt;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{self, str_to_ident, intern_and_get_ident};
use syntax::ptr::P;
use syntax::tokenstream::TokenTree;

fn str_value_of_attr(
    cx: &mut ExtCtxt,
    attr: &ast::Attribute,
    name: &str,
) -> Option<ast::Ident> {
    attr.value_str().map(|value| {
        str_to_ident(&value)
    }).or_else(|| {
        cx.span_err(attr.span(),
            &format!(r#"`{}` must be in the form `#[{}="something"]`"#, name, name));
        None
    })
}

fn single_arg_value_of_attr(
    cx: &mut ExtCtxt,
    attr: &ast::Attribute,
    name: &str,
) -> Option<ast::Ident> {
    let usage_err = || {
        cx.span_err(attr.span(),
            &format!(r#"`{}` must be in the form `#[{}(something)]`"#, name, name));
        None
    };
    // FIXME: This can be cleaned up with slice patterns
    match attr.node.value.node {
        ast::MetaItemKind::List(_, ref items) => {
            if items.len() != 1 {
                return usage_err();
            }
            match items[0].node {
                ast::MetaItemKind::Word(ref value) => Some(str_to_ident(&value)),
                _ => usage_err(),
            }
        }
        _ => usage_err(),
    }
}

pub fn str_value_of_attr_with_name(
    cx: &mut ExtCtxt,
    attrs: &[ast::Attribute],
    name: &str,
) -> Option<ast::Ident> {
    attrs.iter()
        .find(|a| a.check_name(name))
        .and_then(|a| str_value_of_attr(cx, &a, name))
}

pub fn ident_value_of_attr_with_name(
    cx: &mut ExtCtxt,
    attrs: &[ast::Attribute],
    name: &str,
) -> Option<ast::Ident> {
    attrs.iter()
        .find(|a| a.check_name(name))
        .and_then(|a| single_arg_value_of_attr(cx, &a, name))
}

#[cfg(feature = "with-syntex")]
pub fn strip_attributes(krate: ast::Crate) -> ast::Crate {
    use syntax::fold;

    struct StripAttributeFolder;

    impl fold::Folder for StripAttributeFolder {
        fn fold_attribute(&mut self, attr: ast::Attribute) -> Option<ast::Attribute> {
            if attr.check_name("table_name") || attr.check_name("column_name") {
                None
            } else {
                Some(attr)
            }
        }

        fn fold_mac(&mut self, mac: ast::Mac) -> ast::Mac {
            fold::noop_fold_mac(mac, self)
        }
    }

    fold::Folder::fold_crate(&mut StripAttributeFolder, krate)
}

pub fn struct_ty(
    cx: &mut ExtCtxt,
    span: Span,
    name: ast::Ident,
    generics: &ast::Generics,
) -> P<ast::Ty> {
    let lifetimes = generics.lifetimes.iter().map(|lt| lt.lifetime).collect();
    let ty_params = generics.ty_params.iter()
        .map(|param| cx.ty_ident(span, param.ident))
        .collect();
    cx.ty_path(cx.path_all(span, false, vec![name], lifetimes, ty_params, Vec::new()))
}

pub fn ty_param_of_option(ty: &ast::Ty) -> Option<&P<ast::Ty>> {
    match ty.node {
        TyKind::Path(_, ref path) => {
            path.segments.first().iter()
                .filter(|s| s.identifier.name.as_str() == intern_and_get_ident("Option"))
                .flat_map(|s| s.parameters.types().first().map(|p| *p))
                .next()
        }
        _ => None,
    }
}

pub fn is_option_ty(ty: &ast::Ty) -> bool {
    ty_param_of_option(ty).is_some()
}

pub fn lifetime_list_tokens(lifetimes: &[ast::LifetimeDef], span: Span) -> Vec<TokenTree> {
    lifetimes.iter()
        .map(|ld| {
            let name = ld.lifetime.name;
            let lt = token::Lifetime(ast::Ident::with_empty_ctxt(name));
            [TokenTree::Token(span, lt)]
        })
        .collect::<Vec<_>>()
        .join(&TokenTree::Token(span, token::Comma))
}
