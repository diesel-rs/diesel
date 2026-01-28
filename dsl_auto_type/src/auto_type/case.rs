use {heck::*, proc_macro2::Span, syn::Ident};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Case {
    DoNotChange,
    UpperCamel,
    Pascal,
    LowerCamel,
    Snake,
    ShoutySnake,
}

impl Case {
    pub(crate) fn ident_with_case(self, ident: &Ident) -> syn::Ident {
        let s = ident.to_string();
        let cased_s: String = match self {
            Case::DoNotChange => s,
            Case::UpperCamel => s.to_upper_camel_case(),
            Case::Pascal => s.to_pascal_case(),
            Case::LowerCamel => s.to_lower_camel_case(),
            Case::Snake => s.to_snake_case(),
            Case::ShoutySnake => s.to_shouty_snake_case(),
        };
        Ident::new(&cased_s, ident.span())
    }
}

impl Case {
    pub(crate) fn from_str(s: &str, span: Span) -> Result<Self, syn::Error> {
        Ok(match s {
            "dO_nOt_cHaNgE_cAsE" => Case::DoNotChange,
            "UpperCamelCase" => Case::UpperCamel,
            "PascalCase" => Case::Pascal,
            "lowerCamelCase" => Case::LowerCamel,
            "snake_case" => Case::Snake,
            "SHOUTY_SNAKE_CASE" => Case::ShoutySnake,
            other => {
                return Err(syn::Error::new(
                    span,
                    format_args!(
                        "Unknown case: {other}, expected one of: \
                            `PascalCase`, `snake_case`, `UpperCamelCase`, `lowerCamelCase`, \
                            `SHOUTY_SNAKE_CASE`, `dO_nOt_cHaNgE_cAsE`"
                    ),
                ))
            }
        })
    }
}
