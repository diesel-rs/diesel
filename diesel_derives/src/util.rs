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

pub fn ident_value_of_attr_with_name<'a>(attrs: &'a [Attribute], name: &str) -> Option<Ident> {
    let error = || {
        panic!(
            r#"`{}` must be in the form `#[{} = "something"]`"#,
            name, name
        )
    };
    attr_with_name(attrs, name).map(|attr| match attr.value {
        MetaItem::NameValue(_, Lit::Str(ref value, _)) => Ident::from(&**value),
        MetaItem::List(_, ref list) => {
            if list.len() != 1 {
                error();
            }
            println!(
                r#"The form `#[{}(something)]` is deprecated. Use `#[{} = "something"]` instead"#,
                name, name
            );
            if let NestedMetaItem::MetaItem(MetaItem::Word(ref ident)) = list[0] {
                ident.clone()
            } else {
                error()
            }
        }
        _ => error(),
    })
}

pub fn attr_with_name<'a, T>(attrs: T, name: &str) -> Option<&'a Attribute>
where
    T: IntoIterator<Item = &'a Attribute>,
{
    attrs.into_iter().find(|attr| attr.name() == name)
}

pub fn wrap_item_in_const(const_name: Ident, item: Tokens) -> Tokens {
    quote! {
        const #const_name: () = {
            mod diesel {
                __diesel_use_everything!();
            }
            #item
        };
    }
}
