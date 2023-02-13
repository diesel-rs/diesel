use syn::Ident;

#[allow(dead_code)] // paren_token is currently unused
pub struct PrimaryKey {
    paren_token: syn::token::Paren,
    pub keys: syn::punctuated::Punctuated<Ident, syn::Token![,]>,
}

#[allow(dead_code)] // arrow is currently unused
pub struct ColumnDef {
    pub meta: Vec<syn::Attribute>,
    pub column_name: Ident,
    pub sql_name: String,
    arrow: syn::Token![->],
    pub tpe: syn::TypePath,
}

#[allow(dead_code)] // punct and brace_token is currently unused
pub struct TableDecl {
    pub use_statements: Vec<syn::ItemUse>,
    pub meta: Vec<syn::Attribute>,
    pub schema: Option<Ident>,
    punct: Option<syn::Token![.]>,
    pub sql_name: String,
    pub table_name: Ident,
    pub primary_keys: Option<PrimaryKey>,
    brace_token: syn::token::Brace,
    pub column_defs: syn::punctuated::Punctuated<ColumnDef, syn::Token![,]>,
}

#[allow(dead_code)] // eq is currently unused
struct SqlNameAttribute {
    eq: syn::Token![=],
    lit: syn::LitStr,
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
        let meta = syn::Attribute::parse_outer(buf)?;
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
        let (sql_name, meta) = get_sql_name(meta, &table_name)?;
        Ok(Self {
            use_statements,
            meta,
            table_name,
            primary_keys,
            brace_token,
            column_defs,
            sql_name,
            punct,
            schema,
        })
    }
}

impl syn::parse::Parse for PrimaryKey {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = syn::parenthesized!(content in input);
        let keys = content.parse_terminated(Ident::parse)?;
        Ok(Self { paren_token, keys })
    }
}

impl syn::parse::Parse for ColumnDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let meta = syn::Attribute::parse_outer(input)?;
        let column_name = input.parse()?;
        let arrow = input.parse()?;
        let tpe = input.parse()?;
        let (sql_name, meta) = get_sql_name(meta, &column_name)?;
        Ok(Self {
            meta,
            column_name,
            arrow,
            tpe,
            sql_name,
        })
    }
}

impl syn::parse::Parse for SqlNameAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let eq = input.parse()?;
        let lit = input.parse()?;
        Ok(Self { eq, lit })
    }
}

pub fn parse_table_with_schema(
    input: &syn::parse::ParseBuffer<'_>,
) -> Result<(syn::Ident, syn::Token![.], syn::Ident), syn::Error> {
    Ok((input.parse()?, input.parse()?, input.parse()?))
}

fn get_sql_name(
    mut meta: Vec<syn::Attribute>,
    ident: &syn::Ident,
) -> Result<(String, Vec<syn::Attribute>), syn::Error> {
    if let Some(pos) = meta
        .iter()
        .position(|m| m.path.get_ident().map(|i| i == "sql_name").unwrap_or(false))
    {
        let element = meta.remove(pos);
        let inner: SqlNameAttribute = syn::parse2(element.tokens)?;
        Ok((inner.lit.value(), meta))
    } else {
        Ok((ident.to_string(), meta))
    }
}
