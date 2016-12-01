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
) -> Option<String> {
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
    attrs.into_iter().find(|attr| attr.name() == name)
}

fn str_value_of_attr(attr: &Attribute, name: &str) -> String {
    str_value_of_meta_item(&attr.value, name)
}

pub fn str_value_of_meta_item(item: &MetaItem, name: &str) -> String {
    match *item {
        MetaItem::NameValue(_, Lit::Str(ref value, _)) => value.clone(),
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
                NestedMetaItem::MetaItem(MetaItem::Word(ref name)) => name,
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

pub fn inner_of_option_ty(ty: &Ty) -> Option<&Ty> {
    use syn::PathParameters::AngleBracketed;

    if !is_option_ty(ty) {
        return None;
    }

    match *ty {
        Ty::Path(_, Path { ref segments, .. }) =>
            match segments[0].parameters {
                AngleBracketed(ref data) => data.types.first(),
                _ => None,
            },
        _ => None,
    }
}

pub fn get_options_from_input(attrs: &Vec<Attribute>, on_bug: fn() -> !)
    -> Option<Vec<MetaItem>>
{
    let options = attrs.iter().find(|a| a.name() == "options").map(|a| &a.value);
    match options {
        Some(&MetaItem::List(_, ref options)) => {
            Some(options.iter().map(|o| {
                match o {
                   &NestedMetaItem::MetaItem(ref m) => m.clone(),
                   _ => on_bug(),
                }
            }).collect())
        }
        Some(_) => on_bug(),
        None => None,
    }
}

pub fn get_option(
    options: &Vec<MetaItem>,
    option_name: &str,
    on_bug: fn() -> !,
) -> String {
    options.iter().find(|a| a.name() == option_name)
        .map(|a| str_value_of_meta_item(a, option_name))
        .unwrap_or_else(|| on_bug())
}
