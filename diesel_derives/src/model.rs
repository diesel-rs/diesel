use proc_macro2::Span;
use std::slice::from_ref;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Result;
use syn::{
    Data, DataStruct, DeriveInput, Field as SynField, Fields, FieldsNamed, FieldsUnnamed, Ident,
    LitBool, Path, Type,
};

use crate::attrs::{parse_attributes, StructAttr};
use crate::field::Field;
use crate::parsers::{BelongsTo, MysqlType, PostgresType, SqliteType};
use crate::util::camel_to_snake;

pub struct Model {
    name: Path,
    table_names: Vec<Path>,
    pub primary_key_names: Vec<Ident>,
    treat_none_as_default_value: Option<LitBool>,
    treat_none_as_null: Option<LitBool>,
    pub belongs_to: Vec<BelongsTo>,
    pub sql_types: Vec<Type>,
    pub aggregate: bool,
    pub not_sized: bool,
    pub foreign_derive: bool,
    pub mysql_type: Option<MysqlType>,
    pub sqlite_type: Option<SqliteType>,
    pub postgres_type: Option<PostgresType>,
    pub check_for_backend: Option<syn::punctuated::Punctuated<syn::TypePath, syn::Token![,]>>,
    fields: Vec<Field>,
}

impl Model {
    pub fn from_item(
        item: &DeriveInput,
        allow_unit_structs: bool,
        allow_multiple_table: bool,
    ) -> Result<Self> {
        let DeriveInput {
            data, ident, attrs, ..
        } = item;

        let fields = match *data {
            Data::Struct(DataStruct {
                fields: Fields::Named(FieldsNamed { ref named, .. }),
                ..
            }) => Some(named),
            Data::Struct(DataStruct {
                fields: Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }),
                ..
            }) => Some(unnamed),
            _ if !allow_unit_structs => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "This derive can only be used on non-unit structs",
                ));
            }
            _ => None,
        };

        let mut table_names = vec![];
        let mut primary_key_names = vec![Ident::new("id", Span::call_site())];
        let mut treat_none_as_default_value = None;
        let mut treat_none_as_null = None;
        let mut belongs_to = vec![];
        let mut sql_types = vec![];
        let mut aggregate = false;
        let mut not_sized = false;
        let mut foreign_derive = false;
        let mut mysql_type = None;
        let mut sqlite_type = None;
        let mut postgres_type = None;
        let mut check_for_backend = None;

        for attr in parse_attributes(attrs)? {
            match attr.item {
                StructAttr::SqlType(_, value) => sql_types.push(Type::Path(value)),
                StructAttr::TableName(ident, value) => {
                    if !allow_multiple_table && !table_names.is_empty() {
                        return Err(syn::Error::new(
                            ident.span(),
                            "expected a single table name attribute\n\
                             note: remove this attribute",
                        ));
                    }
                    table_names.push(value)
                }
                StructAttr::PrimaryKey(_, keys) => {
                    primary_key_names = keys.into_iter().collect();
                }
                StructAttr::TreatNoneAsDefaultValue(_, val) => {
                    treat_none_as_default_value = Some(val)
                }
                StructAttr::TreatNoneAsNull(_, val) => treat_none_as_null = Some(val),
                StructAttr::BelongsTo(_, val) => belongs_to.push(val),
                StructAttr::Aggregate(_) => aggregate = true,
                StructAttr::NotSized(_) => not_sized = true,
                StructAttr::ForeignDerive(_) => foreign_derive = true,
                StructAttr::MysqlType(_, val) => mysql_type = Some(val),
                StructAttr::SqliteType(_, val) => sqlite_type = Some(val),
                StructAttr::PostgresType(_, val) => postgres_type = Some(val),
                StructAttr::CheckForBackend(_, b) => {
                    check_for_backend = Some(b);
                }
            }
        }

        let name = Ident::new(&infer_table_name(&ident.to_string()), ident.span()).into();

        Ok(Self {
            name,
            table_names,
            primary_key_names,
            treat_none_as_default_value,
            treat_none_as_null,
            belongs_to,
            sql_types,
            aggregate,
            not_sized,
            foreign_derive,
            mysql_type,
            sqlite_type,
            postgres_type,
            fields: fields_from_item_data(fields)?,
            check_for_backend,
        })
    }

    pub fn table_names(&self) -> &[Path] {
        match self.table_names.len() {
            0 => from_ref(&self.name),
            _ => &self.table_names,
        }
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn find_column(&self, column_name: &Ident) -> Result<&Field> {
        self.fields()
            .iter()
            .find(|f| {
                f.column_name()
                    .map(|c| c == *column_name)
                    .unwrap_or_default()
            })
            .ok_or_else(|| {
                syn::Error::new(
                    column_name.span(),
                    format!("No field with column name {column_name}"),
                )
            })
    }

    pub fn treat_none_as_default_value(&self) -> bool {
        self.treat_none_as_default_value
            .as_ref()
            .map(|v| v.value())
            .unwrap_or(true)
    }

    pub fn treat_none_as_null(&self) -> bool {
        self.treat_none_as_null
            .as_ref()
            .map(|v| v.value())
            .unwrap_or(false)
    }
}

fn fields_from_item_data(fields: Option<&Punctuated<SynField, Comma>>) -> Result<Vec<Field>> {
    fields
        .map(|fields| {
            fields
                .iter()
                .enumerate()
                .map(|(i, f)| Field::from_struct_field(f, i))
                .collect::<Result<Vec<_>>>()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

pub fn infer_table_name(name: &str) -> String {
    let mut result = camel_to_snake(name);
    result.push('s');
    result
}

#[test]
fn infer_table_name_pluralizes_and_downcases() {
    assert_eq!("foos", &infer_table_name("Foo"));
    assert_eq!("bars", &infer_table_name("Bar"));
}

#[test]
fn infer_table_name_properly_handles_underscores() {
    assert_eq!("foo_bars", &infer_table_name("FooBar"));
    assert_eq!("foo_bar_bazs", &infer_table_name("FooBarBaz"));
}
