use std::mem;
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

pub fn list_value_of_attr_with_name<'a>(
    attrs: &'a [Attribute],
    name: &str,
) -> Option<Vec<&'a Ident>> {
    attr_with_name(attrs, name).map(|attr| list_value_of_attr(attr, name))
}

pub fn attr_with_name<'a>(
    attrs: &'a [Attribute],
    name: &str,
) -> Option<&'a Attribute> {
    attrs.into_iter().find(|attr| attr.name() == name)
}

fn str_value_of_attr<'a>(attr: &'a Attribute, name: &str) -> &'a str {
    str_value_of_meta_item(&attr.value, name)
}

pub fn str_value_of_meta_item<'a>(item: &'a MetaItem, name: &str) -> &'a str {
    match *item {
        MetaItem::NameValue(_, Lit::Str(ref value, _)) => &*value,
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

fn list_value_of_attr<'a>(attr: &'a Attribute, name: &str) -> Vec<&'a Ident> {
    match attr.value {
        MetaItem::List(_, ref items) => {
            items.iter().map(|item| match *item {
                MetaItem::Word(ref name) => name,
                _ => panic!("`{}` must be in the form `#[{}(something, something_else)]`", name, name),
            }).collect()
        }
        _ => panic!("`{}` must be in the form `#[{}(something, something_else)]`", name, name),
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

pub fn strip_attributes(attrs: Vec<Attribute>, names_to_strip: &[&str]) -> Vec<Attribute> {
    attrs.into_iter().filter(|attr| {
        !names_to_strip.contains(&attr.name())
    }).collect()
}

pub fn strip_field_attributes(item: &mut MacroInput, names_to_strip: &[&str]) {
    let fields = match item.body {
        Body::Struct(VariantData::Struct(ref mut fields)) |
        Body::Struct(VariantData::Tuple(ref mut fields)) => fields,
        _ => return,
    };

    let mut attrs = Vec::new();
    for field in fields {
        mem::swap(&mut attrs, &mut field.attrs);
        attrs = strip_attributes(attrs, names_to_strip);
        mem::swap(&mut attrs, &mut field.attrs);
    }
}

pub fn get_options_from_input(attrs: &[Attribute], on_bug: fn() -> !)
    -> Option<&[MetaItem]>
{
    let options = attrs.iter().find(|a| a.name() == "options").map(|a| &a.value);
    match options {
        Some(&MetaItem::List(_, ref options)) => Some(options),
        Some(_) => on_bug(),
        None => None,
    }
}

pub fn get_option<'a>(
    options: &'a [MetaItem],
    option_name: &str,
    on_bug: fn() -> !,
) -> &'a str {
    options.iter().find(|a| a.name() == option_name)
        .map(|a| str_value_of_meta_item(a, option_name))
        .unwrap_or_else(|| on_bug())
}
