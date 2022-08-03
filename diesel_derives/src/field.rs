use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned;
use syn::{Field as SynField, Ident, Index, Type};

use attrs::{parse_attributes, AttributeSpanWrapper, FieldAttr, SqlIdentifier};

pub struct Field {
    pub ty: Type,
    pub span: Span,
    pub name: FieldName,
    column_name: Option<AttributeSpanWrapper<SqlIdentifier>>,
    pub sql_type: Option<AttributeSpanWrapper<Type>>,
    pub serialize_as: Option<AttributeSpanWrapper<Type>>,
    pub deserialize_as: Option<AttributeSpanWrapper<Type>>,
    pub select_expression: Option<AttributeSpanWrapper<SelectExpr>>,
    pub select_expression_type: Option<AttributeSpanWrapper<Type>>,
    pub embed: Option<AttributeSpanWrapper<bool>>,
}

impl Field {
    pub fn from_struct_field(field: &SynField, index: usize) -> Self {
        let SynField {
            ident, attrs, ty, ..
        } = field;

        let mut column_name = None;
        let mut sql_type = None;
        let mut serialize_as = None;
        let mut deserialize_as = None;
        let mut embed = None;
        let mut select_expression = None;
        let mut select_expression_type = None;

        for attr in parse_attributes(attrs) {
            let attribute_span = attr.attribute_span;
            let ident_span = attr.ident_span;
            match attr.item {
                FieldAttr::ColumnName(_, value) => {
                    column_name = Some(AttributeSpanWrapper {
                        item: value,
                        attribute_span,
                        ident_span,
                    })
                }
                FieldAttr::SqlType(_, value) => {
                    sql_type = Some(AttributeSpanWrapper {
                        item: Type::Path(value),
                        attribute_span,
                        ident_span,
                    })
                }
                FieldAttr::SerializeAs(_, value) => {
                    serialize_as = Some(AttributeSpanWrapper {
                        item: Type::Path(value),
                        attribute_span,
                        ident_span,
                    })
                }
                FieldAttr::DeserializeAs(_, value) => {
                    deserialize_as = Some(AttributeSpanWrapper {
                        item: Type::Path(value),
                        attribute_span,
                        ident_span,
                    })
                }
                FieldAttr::SelectExpression(_, value) => {
                    select_expression = Some(AttributeSpanWrapper {
                        item: value,
                        attribute_span,
                        ident_span,
                    })
                }
                FieldAttr::SelectExpressionType(_, value) => {
                    select_expression_type = Some(AttributeSpanWrapper {
                        item: value,
                        attribute_span,
                        ident_span,
                    })
                }
                FieldAttr::Embed(_) => {
                    embed = Some(AttributeSpanWrapper {
                        item: true,
                        attribute_span,
                        ident_span,
                    })
                }
            }
        }

        let name = match ident.clone() {
            Some(x) => FieldName::Named(x),
            None => FieldName::Unnamed(index.into()),
        };

        let span = match name {
            FieldName::Named(ref ident) => ident.span(),
            FieldName::Unnamed(_) => ty.span(),
        };

        Self {
            ty: ty.clone(),
            span,
            name,
            column_name,
            sql_type,
            serialize_as,
            deserialize_as,
            select_expression,
            select_expression_type,
            embed,
        }
    }

    pub fn column_name(&self) -> SqlIdentifier {
        self.column_name
            .as_ref()
            .map(|a| a.item.clone())
            .unwrap_or_else(|| match self.name {
                FieldName::Named(ref x) => x.into(),
                FieldName::Unnamed(ref x) => {
                    abort!(
                    x,
                    "All fields of tuple structs must be annotated with `#[diesel(column_name)]`"
                );
                }
            })
    }

    pub fn ty_for_deserialize(&self) -> &Type {
        if let Some(AttributeSpanWrapper { item: value, .. }) = &self.deserialize_as {
            value
        } else {
            &self.ty
        }
    }

    pub(crate) fn embed(&self) -> bool {
        self.embed.as_ref().map(|a| a.item).unwrap_or(false)
    }
}

pub enum FieldName {
    Named(Ident),
    Unnamed(Index),
}

impl quote::ToTokens for FieldName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            FieldName::Named(ref x) => x.to_tokens(tokens),
            FieldName::Unnamed(ref x) => x.to_tokens(tokens),
        }
    }
}

pub enum SelectExpr {
    Expr(syn::Expr),
    Tuple {
        paren_token: syn::token::Paren,
        content: proc_macro2::TokenStream,
    },
}

impl quote::ToTokens for SelectExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            SelectExpr::Expr(ref e) => e.to_tokens(tokens),
            SelectExpr::Tuple {
                ref paren_token,
                ref content,
            } => paren_token.surround(tokens, |tokens| content.to_tokens(tokens)),
        }
    }
}

impl syn::parse::Parse for SelectExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(syn::token::Paren) {
            let content;
            let paren_token = syn::parenthesized!(content in input);
            Ok(Self::Tuple {
                paren_token,
                content: content.parse()?,
            })
        } else {
            input.parse::<syn::Expr>().map(Self::Expr)
        }
    }
}
