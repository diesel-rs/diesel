use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    Data, DataStruct, DeriveInput, Field as SynField, Fields, FieldsNamed, FieldsUnnamed, Ident,
    LitBool, Path, Type,
};

use attrs::{parse_attributes, StructAttr};
use field::Field;
use parsers::{BelongsTo, MysqlType, PostgresType, SqliteType};
use util::camel_to_snake;

pub struct Model {
    name: Path,
    table_name: Option<Path>,
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
    fields: Vec<Field>,
}

impl Model {
    pub fn from_item(item: &DeriveInput, allow_unit_structs: bool) -> Self {
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
                abort_call_site!("This derive can only be used on non-unit structs")
            }
            _ => None,
        };

        let mut table_name = None;
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

        for attr in parse_attributes(attrs) {
            match attr {
                StructAttr::SqlType(_, value) => sql_types.push(value),
                StructAttr::TableName(_, value) => table_name = Some(value),
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
            }
        }

        let name = Ident::new(&infer_table_name(&ident.to_string()), ident.span()).into();

        Self {
            name,
            table_name,
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
            fields: fields_from_item_data(fields),
        }
    }

    pub fn table_name(&self) -> &Path {
        self.table_name.as_ref().unwrap_or(&self.name)
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn find_column(&self, column_name: &Ident) -> &Field {
        self.fields()
            .iter()
            .find(|f| f.column_name() == *column_name)
            .unwrap_or_else(|| abort!(column_name, "No field with column name {}", column_name))
    }

    pub fn has_table_name_attribute(&self) -> bool {
        self.table_name.is_some()
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

fn fields_from_item_data(fields: Option<&Punctuated<SynField, Comma>>) -> Vec<Field> {
    fields
        .map(|fields| {
            fields
                .iter()
                .enumerate()
                .map(|(i, f)| Field::from_struct_field(f, i))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
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
