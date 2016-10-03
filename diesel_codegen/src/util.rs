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
    attrs.into_iter().find(|attr| attr.name() == name)
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
