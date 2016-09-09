use syn::*;

use ast_builder::ty_ident;

pub fn struct_ty(name: Ident, generics: &Generics) -> Ty {
    let lifetimes = generics.lifetimes.iter().map(|lt| lt.lifetime.clone()).collect();
    let ty_params = generics.ty_params.iter()
        .map(|param| ty_ident(param.ident.clone()))
        .collect();
    let parameter_data = AngleBracketedParameterData {
        lifetimes: lifetimes,
        types: ty_params,
        bindings: Vec::new(),
    };
    let parameters = PathParameters::AngleBracketed(parameter_data);
    Ty::Path(None, Path {
        global: false,
        segments: vec![
            PathSegment {
                ident: name,
                parameters: parameters,
            },
        ],
    })
}

pub fn str_value_of_attr_with_name<'a>(
    attrs: &'a [Attribute],
    name: &str,
) -> Option<&'a str> {
    attr_with_name(attrs, name).map(|attr| str_value_of_attr(attr, name))
}

pub fn ident_value_of_attr_with_name<'a>(
    attrs: &'a [Attribute],
    name: &str,
) -> Option<&'a Ident> {
    attr_with_name(attrs, name).map(|attr| single_arg_value_of_attr(attr, name))
}

pub fn attr_with_name<'a>(
    attrs: &'a [Attribute],
    name: &str,
) -> Option<&'a Attribute> {
    let ident_name = Ident::new(name);
    attrs.into_iter().find(|attr| *attr_name(attr) == ident_name)
}

pub fn attr_name(attr: &Attribute) -> &Ident {
    match attr.value {
        MetaItem::Word(ref name) => name,
        MetaItem::List(ref name, _) => name,
        MetaItem::NameValue(ref name, _) => name,
    }
}

fn str_value_of_attr<'a>(attr: &'a Attribute, name: &str) -> &'a str {
    match attr.value {
        MetaItem::NameValue(_, ref value) => &*value,
        _ => panic!(r#"`{}` must be in the form `#[{}="something"]`"#, name, name),
    }
}

fn single_arg_value_of_attr<'a>(attr: &'a Attribute, name: &str) -> &'a Ident {
    let usage_err = || panic!(r#"`{}` must be in the form `#[{}(something)]`"#, name, name);
    match attr.value {
        MetaItem::List(_, ref items) => {
            if items.len() != 1 {
                return usage_err();
            }
            match items[0] {
                MetaItem::Word(ref name) => name,
                _ => usage_err(),
            }
        }
        _ => usage_err(),
    }
}

pub fn is_option_ty(ty: &Ty) -> bool {
    let option_ident = Ident::new("Option");
    match *ty {
        Ty::Path(_, ref path) => {
            path.segments.first()
                .map(|s| s.ident == option_ident)
                .unwrap_or(false)
        }
        _ => false,
    }
}
