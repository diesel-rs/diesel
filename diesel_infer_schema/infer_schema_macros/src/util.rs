use syn::*;

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

pub fn get_options_from_input(
    name: &str,
    attrs: &[Attribute],
    on_bug: fn() -> !,
) -> Option<Vec<MetaItem>> {
    let options = attrs.iter().find(|a| a.name() == name).map(|a| &a.value);
    match options {
        Some(&MetaItem::List(_, ref options)) => Some(
            options
                .iter()
                .map(|o| match *o {
                    NestedMetaItem::MetaItem(ref m) => m.clone(),
                    _ => on_bug(),
                })
                .collect(),
        ),
        Some(_) => on_bug(),
        None => None,
    }
}

pub fn get_option<'a>(options: &'a [MetaItem], option_name: &str, on_bug: fn() -> !) -> &'a str {
    get_optional_option(options, option_name).unwrap_or_else(|| on_bug())
}

pub fn get_optional_option<'a>(options: &'a [MetaItem], option_name: &str) -> Option<&'a str> {
    options
        .iter()
        .find(|a| a.name() == option_name)
        .map(|a| str_value_of_meta_item(a, option_name))
}
