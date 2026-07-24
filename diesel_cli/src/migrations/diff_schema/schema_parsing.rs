use crate::infer_schema_internals::{ColumnDefinition, ColumnType, QueryRelationData};
use diesel_attribute_parser::FieldAttr;
use diesel_attribute_parser::StructAttr;
use diesel_attribute_parser::parsers::PostgresType;
use diesel_table_macro_syntax::TableDecl;

#[derive(Clone)]
pub(crate) struct Joinable {
    pub(crate) parent_table: syn::Ident,
    pub(crate) child_table: syn::Ident,
    pub(crate) ref_column: syn::Ident,
}

impl syn::parse::Parse for Joinable {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let child_table = input.parse()?;
        let _arrow: syn::Token![->] = input.parse()?;
        let parent_table = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let ref_column = content.parse()?;
        Ok(Self {
            child_table,
            parent_table,
            ref_column,
        })
    }
}

#[derive(Default)]
pub(crate) struct SchemaCollector {
    pub(crate) table_decls: Vec<Result<TableDecl, syn::Error>>,
    pub(crate) joinable: Vec<Result<Joinable, syn::Error>>,
    pub(crate) enums: Vec<EnumInfos>,
    pub(crate) enum_sql_types: Vec<SqlTypeInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumInfos {
    pub(crate) variants: Vec<EnumVariant>,
    pub(crate) sql_type: syn::TypePath,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumVariant {
    pub(crate) sql_name: String,
}

#[derive(Debug, Clone)]
pub struct PostgresSqlTypeInfo {
    pub(crate) name: String,
    pub(crate) schema: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SqlTypeInfo {
    pub(crate) postgres_enum: Option<PostgresSqlTypeInfo>,
    pub(crate) mysql_enum: bool,
    pub(crate) rust_name: syn::Ident,
}

impl SqlTypeInfo {
    pub(crate) fn is_type(
        &self,
        c: &ColumnType,
        rust_side_schema: &SchemaCollector,
        col: &ColumnDefinition,
        tab: &QueryRelationData,
    ) -> bool {
        if let Some(pg_type) = &self.postgres_enum
            && pg_type.name == c.sql_name
            && pg_type.schema == c.schema
        {
            true
        } else if self.mysql_enum {
            let rust_side_table = rust_side_schema.table_decls.iter().find_map(|t| {
                if let Ok(t) = t {
                    (t.view.sql_name == tab.table_name().sql_name).then_some(t)
                } else {
                    None
                }
            });
            let rust_side_column = rust_side_table.and_then(|t| {
                t.view
                    .column_defs
                    .iter()
                    .find(|c| c.sql_name == col.sql_name)
            });
            if let Some(rust_side_column) = rust_side_column {
                Some(&self.rust_name) == rust_side_column.tpe.path.segments.last().map(|i| &i.ident)
            } else {
                false
            }
        } else {
            false
        }
    }

    pub(super) fn from_column_type(c: &ColumnType) -> Result<Self, crate::errors::Error> {
        Ok(Self {
            postgres_enum: Some(PostgresSqlTypeInfo {
                name: c.sql_name.clone(),
                schema: c.schema.clone(),
            }),
            mysql_enum: true,
            rust_name: syn::parse_str(&c.rust_name)?,
        })
    }
}

impl PostgresSqlTypeInfo {
    #[cfg(feature = "postgres")]
    pub(crate) fn type_name<'b>(
        &self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, diesel::pg::Pg>,
    ) -> diesel::QueryResult<()> {
        if let Some(schema) = self.schema.as_deref() {
            pass.push_identifier(schema)?;
            pass.push_sql(".");
        }
        pass.push_identifier(&self.name)?;
        Ok(())
    }
}

impl<'ast> syn::visit::Visit<'ast> for SchemaCollector {
    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        let last_segment = i.path.segments.last();
        if last_segment.map(|s| s.ident == "table").unwrap_or(false) {
            self.table_decls.push(i.parse_body());
        } else if last_segment.map(|s| s.ident == "joinable").unwrap_or(false) {
            self.joinable.push(i.parse_body());
        }
        syn::visit::visit_macro(self, i)
    }

    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        let found_derive = has_derive(&i.attrs, "Enum") && has_derive(&i.attrs, "Debug");
        let mut sql_type = None;
        let mut rename_all = None;
        match diesel_attribute_parser::parse_attributes::<diesel_attribute_parser::StructAttr>(
            &i.attrs,
        ) {
            Ok(attrs) => {
                sql_type = attrs.iter().find_map(|a| {
                    if let StructAttr::SqlType(_, p) = &a.item {
                        Some(p.clone())
                    } else {
                        None
                    }
                });
                rename_all = attrs.iter().find_map(|a| {
                    if let StructAttr::RenameAll(_, a) = &a.item {
                        Some(*a)
                    } else {
                        None
                    }
                });
            }
            Err(e) => {
                tracing::warn!("Failed to parse attributes: {e}");
            }
        };

        let mut variant_less_enum = true;
        let mut variants = Vec::new();
        for v in &i.variants {
            if v.fields.is_empty() {
                let rename_attr = diesel_attribute_parser::parse_attributes::<
                    diesel_attribute_parser::FieldAttr,
                >(&v.attrs)
                .inspect_err(|e| tracing::warn!("Failed to parse attributes: {e}"))
                .unwrap_or_default()
                .into_iter()
                .find_map(|n| {
                    if let FieldAttr::Rename(_, n) = n.item {
                        Some(n.value())
                    } else {
                        None
                    }
                });
                let name = if let Some(a) = rename_attr {
                    a
                } else {
                    let name = v.ident.to_string();
                    match rename_all {
                        None => name,
                        Some(t) => t.apply_case_to_enum_variant(name),
                    }
                };
                variants.push(EnumVariant { sql_name: name });
            } else {
                variant_less_enum = false;
                break;
            }
        }
        if variant_less_enum
            && found_derive
            && let Some(sql_type) = sql_type
        {
            self.enums.push(EnumInfos { variants, sql_type });
        }

        syn::visit::visit_item_enum(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        if i.fields.is_empty() && has_derive(&i.attrs, "SqlType") {
            let attrs = diesel_attribute_parser::parse_attributes::<StructAttr>(&i.attrs)
                .inspect_err(|e| tracing::info!("Failed to parse attributes: {e}"))
                .unwrap_or_default();
            let has_enum_attr = attrs
                .iter()
                .any(|a| matches!(a.item, StructAttr::EnumType(_)));
            let pg_type = attrs.iter().find_map(|e| {
                if let StructAttr::PostgresType(_, t) = &e.item {
                    Some(t)
                } else {
                    None
                }
            });
            let mysql_enum = attrs.iter().any(
                |c| matches!(&c.item, StructAttr::MysqlType(_, t) if t.name.value() == "Enum"),
            );
            if has_enum_attr {
                let pg_enum = if let Some(PostgresType::Lookup(name, schema)) = pg_type {
                    Some(PostgresSqlTypeInfo {
                        name: name.value(),
                        schema: schema.as_ref().map(|v| v.value()),
                    })
                } else {
                    None
                };
                self.enum_sql_types.push(SqlTypeInfo {
                    postgres_enum: pg_enum,
                    mysql_enum,
                    rust_name: i.ident.clone(),
                });
            }
        }

        syn::visit::visit_item_struct(self, i);
    }
}

pub(crate) fn has_derive(attrs: &[syn::Attribute], derive: &str) -> bool {
    attrs.iter().any(|a| {
        if let syn::Meta::List(m) = &a.meta
            && m.path.is_ident("derive")
            && let Ok(derives) = m.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
        {
            return derives.iter().any(|d| {
                d.segments
                    .last()
                    .map(|l| l.ident == derive)
                    .unwrap_or_default()
            });
        }

        false
    })
}
