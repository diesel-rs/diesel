use syntax::ast;
use syntax::attr::AttrMetaMethods;
use syntax::ext::base::ExtCtxt;
use syntax::parse::token::str_to_ident;

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

pub fn str_value_of_attr_with_name(
    cx: &mut ExtCtxt,
    attrs: &[ast::Attribute],
    name: &str,
) -> Option<ast::Ident> {
    attrs.iter()
        .find(|a| a.check_name(name))
        .and_then(|a| str_value_of_attr(cx, &a, name))
}

#[cfg(feature = "with-syntex")]
pub fn strip_attributes(krate: ast::Crate) -> ast::Crate {
    use syntax::fold;

    struct StripAttributeFolder;

    impl fold::Folder for StripAttributeFolder {
        fn fold_attribute(&mut self, attr: ast::Attribute) -> Option<ast::Attribute> {
            if attr.check_name("table_name") {
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
