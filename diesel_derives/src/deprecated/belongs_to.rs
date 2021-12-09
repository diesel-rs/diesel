use syn::parse::{ParseStream, Result};
use syn::token::Comma;
use syn::{parenthesized, Ident, LitStr};

use deprecated::utils::parse_eq_and_lit_str;
use parsers::BelongsTo;
use util::BELONGS_TO_NOTE;

pub fn parse_belongs_to(name: Ident, input: ParseStream) -> Result<BelongsTo> {
    if input.is_empty() {
        abort!(
            name.span(),
            "unexpected end of input, expected parentheses";
            help = "The correct format looks like `#[diesel({})]`", BELONGS_TO_NOTE
        );
    }

    let content;
    parenthesized!(content in input);

    let parent = if content.peek(Ident) {
        let name: Ident = content.parse()?;

        if name == "parent" {
            let lit_str = parse_eq_and_lit_str(name, &content, BELONGS_TO_NOTE)?;
            lit_str.parse()?
        } else {
            LitStr::new(&name.to_string(), name.span()).parse()?
        }
    } else {
        content.parse()?
    };

    let mut foreign_key = None;

    if content.peek(Comma) {
        content.parse::<Comma>()?;

        let name: Ident = content.parse()?;

        if name != "foreign_key" {
            abort!(name, "expected `foreign_key`");
        }

        let lit_str = parse_eq_and_lit_str(name, &content, BELONGS_TO_NOTE)?;
        foreign_key = Some(lit_str.parse()?);
    }

    Ok(BelongsTo {
        parent,
        foreign_key,
    })
}
