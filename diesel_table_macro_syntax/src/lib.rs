use syn::spanned::Spanned;
use syn::Ident;
use syn::MetaNameValue;

pub struct TableDecl {
    pub use_statements: Vec<syn::ItemUse>,
    pub meta: Vec<syn::Attribute>,
    pub schema: Option<Ident>,
    _punct: Option<syn::Token![.]>,
    pub sql_name: String,
    pub table_name: Ident,
    pub primary_keys: Option<PrimaryKey>,
    _brace_token: syn::token::Brace,
    pub column_defs: syn::punctuated::Punctuated<ColumnDef, syn::Token![,]>,
}

#[allow(dead_code)] // paren_token is currently unused
pub struct PrimaryKey {
    paren_token: syn::token::Paren,
    pub keys: syn::punctuated::Punctuated<Ident, syn::Token![,]>,
}

pub struct ColumnDef {
    pub meta: Vec<syn::Attribute>,
    pub column_name: Ident,
    pub sql_name: String,
    _arrow: syn::Token![->],
    pub tpe: syn::TypePath,
    pub max_length: Option<syn::LitInt>,
}

impl syn::parse::Parse for TableDecl {
    fn parse(buf: &syn::parse::ParseBuffer<'_>) -> Result<Self, syn::Error> {
        let mut use_statements = Vec::new();
        loop {
            let fork = buf.fork();
            if fork.parse::<syn::ItemUse>().is_ok() {
                use_statements.push(buf.parse()?);
            } else {
                break;
            };
        }
        let mut meta = syn::Attribute::parse_outer(buf)?;
        let fork = buf.fork();
        let (schema, punct, table_name) = if parse_table_with_schema(&fork).is_ok() {
            let (schema, punct, table_name) = parse_table_with_schema(buf)?;
            (Some(schema), Some(punct), table_name)
        } else {
            let table_name = buf.parse()?;
            (None, None, table_name)
        };
        let fork = buf.fork();
        let primary_keys = if fork.parse::<PrimaryKey>().is_ok() {
            Some(buf.parse()?)
        } else {
            None
        };
        let content;
        let brace_token = syn::braced!(content in buf);
        let column_defs = syn::punctuated::Punctuated::parse_terminated(&content)?;
        let sql_name = get_sql_name(&mut meta, &table_name)?;
        Ok(Self {
            use_statements,
            meta,
            table_name,
            primary_keys,
            _brace_token: brace_token,
            column_defs,
            sql_name,
            _punct: punct,
            schema,
        })
    }
}

impl syn::parse::Parse for PrimaryKey {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = syn::parenthesized!(content in input);
        let keys = content.parse_terminated(Ident::parse, syn::Token![,])?;
        Ok(Self { paren_token, keys })
    }
}

impl syn::parse::Parse for ColumnDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut meta = syn::Attribute::parse_outer(input)?;
        let column_name: syn::Ident = input.parse()?;
        let _arrow: syn::Token![->] = input.parse()?;
        let tpe: syn::TypePath = input.parse()?;

        let sql_name = get_sql_name(&mut meta, &column_name)?;
        let max_length = take_lit(&mut meta, "max_length", |lit| match lit {
            syn::Lit::Int(lit_int) => Some(lit_int),
            _ => None,
        })?;

        Ok(Self {
            meta,
            column_name,
            _arrow,
            tpe,
            max_length,
            sql_name,
        })
    }
}

pub fn parse_table_with_schema(
    input: &syn::parse::ParseBuffer<'_>,
) -> Result<(syn::Ident, syn::Token![.], syn::Ident), syn::Error> {
    Ok((input.parse()?, input.parse()?, input.parse()?))
}

fn get_sql_name(
    meta: &mut Vec<syn::Attribute>,
    fallback_ident: &syn::Ident,
) -> Result<String, syn::Error> {
    Ok(
        match take_lit(meta, "sql_name", |lit| match lit {
            syn::Lit::Str(lit_str) => Some(lit_str),
            _ => None,
        })? {
            None => {
                use syn::ext::IdentExt;
                fallback_ident.unraw().to_string()
            }
            Some(str_lit) => {
                let mut str_lit = str_lit.value();
                if str_lit.starts_with("r#") {
                    str_lit.drain(..2);
                }
                str_lit
            }
        },
    )
}

fn take_lit<O, F>(
    meta: &mut Vec<syn::Attribute>,
    attribute_name: &'static str,
    extraction_fn: F,
) -> Result<Option<O>, syn::Error>
where
    F: FnOnce(syn::Lit) -> Option<O>,
{
    if let Some(index) = meta.iter().position(|m| {
        m.path()
            .get_ident()
            .map(|i| i == attribute_name)
            .unwrap_or(false)
    }) {
        let attribute = meta.remove(index);
        let span = attribute.span();
        let extraction_after_finding_attr = if let syn::Meta::NameValue(MetaNameValue {
            value: syn::Expr::Lit(syn::ExprLit { lit, .. }),
            ..
        }) = attribute.meta
        {
            extraction_fn(lit)
        } else {
            None
        };
        return Ok(Some(extraction_after_finding_attr.ok_or_else(|| {
            syn::Error::new(
                span,
                format_args!("Invalid `#[sql_name = {attribute_name:?}]` attribute"),
            )
        })?));
    }
    Ok(None)
}
