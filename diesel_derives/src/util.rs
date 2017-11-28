use quote::Tokens;
use syn::*;

use ast_builder::ty_ident;

pub fn struct_ty(name: Ident, generics: &Generics) -> Ty {
    let lifetimes = generics
        .lifetimes
        .iter()
        .map(|lt| lt.lifetime.clone())
        .collect();
    let ty_params = generics
        .ty_params
        .iter()
        .map(|param| ty_ident(param.ident.clone()))
        .collect();
    let parameter_data = AngleBracketedParameterData {
        lifetimes: lifetimes,
        types: ty_params,
        bindings: Vec::new(),
    };
    let parameters = PathParameters::AngleBracketed(parameter_data);
    Ty::Path(
        None,
        Path {
            global: false,
            segments: vec![
                PathSegment {
                    ident: name,
                    parameters: parameters,
                },
            ],
        },
    )
}

pub fn str_value_of_attr_with_name<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a str> {
    attr_with_name(attrs, name).map(|attr| str_value_of_attr(attr, name))
}

pub fn ident_value_of_attr_with_name<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Ident> {
    list_value_of_attr_with_name(attrs, name).map(|idents| {
        if idents.len() != 1 {
            panic!(r#"`{}` must be in the form `#[{}(something)]`"#, name, name);
        }
        idents[0]
    })
}

pub fn list_value_of_attr_with_name<'a>(
    attrs: &'a [Attribute],
    name: &str,
) -> Option<Vec<&'a Ident>> {
    attr_with_name(attrs, name).map(|attr| list_value_of_attr(attr, name))
}

pub fn attr_with_name<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attrs.into_iter().find(|attr| attr.name() == name)
}

fn str_value_of_attr<'a>(attr: &'a Attribute, name: &str) -> &'a str {
    str_value_of_meta_item(&attr.value, name)
}

pub fn str_value_of_meta_item<'a>(item: &'a MetaItem, name: &str) -> &'a str {
    match *item {
        MetaItem::NameValue(_, Lit::Str(ref value, _)) => &*value,
        _ => panic!(
            r#"`{}` must be in the form `#[{}="something"]`"#,
            name,
            name
        ),
    }
}

fn list_value_of_attr<'a>(attr: &'a Attribute, name: &str) -> Vec<&'a Ident> {
    match attr.value {
        MetaItem::List(_, ref items) => items
            .iter()
            .map(|item| match *item {
                NestedMetaItem::MetaItem(MetaItem::Word(ref name)) => name,
                _ => panic!(r#"`{}` must be in the form `#[{}(something)]`"#, name, name),
            })
            .collect(),
        _ => panic!(r#"`{}` must be in the form `#[{}(something)]`"#, name, name),
    }
}

pub fn is_option_ty(ty: &Ty) -> bool {
    let option_ident = Ident::new("Option");
    match *ty {
        Ty::Path(_, ref path) => path.segments
            .first()
            .map(|s| s.ident == option_ident)
            .unwrap_or(false),
        _ => false,
    }
}

pub fn inner_of_option_ty(ty: &Ty) -> Option<&Ty> {
    use syn::PathParameters::AngleBracketed;

    if !is_option_ty(ty) {
        return None;
    }

    match *ty {
        Ty::Path(_, Path { ref segments, .. }) => match segments[0].parameters {
            AngleBracketed(ref data) => data.types.first(),
            _ => None,
        },
        _ => None,
    }
}

pub fn wrap_item_in_const(const_name: Ident, item: Tokens) -> Tokens {
    quote! {
        const #const_name: () = {
            extern crate diesel;
            #item
        };
    }
}
