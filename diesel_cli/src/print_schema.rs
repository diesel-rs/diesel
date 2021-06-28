use crate::config;
use crate::database::Backend;
use crate::infer_schema_internals::*;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_regex::Serde as RegexWrapper;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};
use std::io::Write as IoWrite;

const SCHEMA_HEADER: &str = "// @generated automatically by Diesel CLI.\n";

type Regex = RegexWrapper<::regex::Regex>;

pub enum Filtering {
    OnlyTables(Vec<Regex>),
    ExceptTables(Vec<Regex>),
    None,
}

impl Default for Filtering {
    fn default() -> Self {
        Filtering::None
    }
}

impl Filtering {
    pub fn should_ignore_table(&self, name: &TableName) -> bool {
        use self::Filtering::*;

        match *self {
            OnlyTables(ref regexes) => !regexes.iter().any(|regex| regex.is_match(&name.sql_name)),
            ExceptTables(ref regexes) => regexes.iter().any(|regex| regex.is_match(&name.sql_name)),
            None => false,
        }
    }
}

/// How to sort columns when querying the table schema.
#[derive(Debug, Deserialize, Serialize)]
pub enum ColumnSorting {
    /// Order by ordinal position
    #[serde(rename = "ordinal_position")]
    OrdinalPosition,
    /// Order by column name
    #[serde(rename = "name")]
    Name,
}

impl Default for ColumnSorting {
    fn default() -> Self {
        ColumnSorting::OrdinalPosition
    }
}

pub fn run_print_schema<W: IoWrite>(
    database_url: &str,
    config: &config::PrintSchema,
    output: &mut W,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let schema = output_schema(database_url, config)?;

    output.write_all(schema.as_bytes())?;
    Ok(())
}

fn common_diesel_types(types: &mut HashSet<&str>) {
    types.insert("Bool");
    types.insert("Integer");
    types.insert("SmallInt");
    types.insert("BigInt");
    types.insert("Binary");
    types.insert("Text");
    types.insert("Double");
    types.insert("Float");
    types.insert("Numeric");
    types.insert("Timestamp");
    types.insert("Date");
    types.insert("Time");

    // hidden type defs
    types.insert("Float4");
    types.insert("Smallint");
    types.insert("Int2");
    types.insert("Int4");
    types.insert("Int8");
    types.insert("Bigint");
    types.insert("Float8");
    types.insert("Decimal");
    types.insert("VarChar");
    types.insert("Varchar");
    types.insert("Char");
    types.insert("Tinytext");
    types.insert("Mediumtext");
    types.insert("Longtext");
    types.insert("Tinyblob");
    types.insert("Blob");
    types.insert("Mediumblob");
    types.insert("Longblob");
    types.insert("Varbinary");
    types.insert("Bit");
}

#[cfg(feature = "postgres")]
fn pg_diesel_types() -> HashSet<&'static str> {
    let mut types = HashSet::new();
    types.insert("Cidr");
    types.insert("Inet");
    types.insert("Jsonb");
    types.insert("MacAddr");
    types.insert("Money");
    types.insert("Oid");
    types.insert("Range");
    types.insert("Timestamptz");
    types.insert("Uuid");
    types.insert("Json");
    types.insert("Record");
    types.insert("Interval");

    // hidden type defs
    types.insert("Int4range");
    types.insert("Int8range");
    types.insert("Daterange");
    types.insert("Numrange");
    types.insert("Tsrange");
    types.insert("Tstzrange");
    types.insert("SmallSerial");
    types.insert("BigSerial");
    types.insert("Serial");
    types.insert("Bytea");
    types.insert("Bpchar");
    types.insert("Macaddr");

    common_diesel_types(&mut types);
    types
}

#[cfg(feature = "mysql")]
fn mysql_diesel_types() -> HashSet<&'static str> {
    let mut types = HashSet::new();
    common_diesel_types(&mut types);

    types.insert("TinyInt");
    types.insert("Tinyint");
    types
}

#[cfg(feature = "sqlite")]
fn sqlite_diesel_types() -> HashSet<&'static str> {
    let mut types = HashSet::new();
    common_diesel_types(&mut types);
    types
}

pub fn output_schema(
    database_url: &str,
    config: &config::PrintSchema,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    let table_names = load_table_names(database_url, config.schema_name())?
        .into_iter()
        .filter(|t| !config.filter.should_ignore_table(t))
        .collect::<Vec<_>>();
    let foreign_keys = load_foreign_key_constraints(database_url, config.schema_name())?;
    let foreign_keys =
        remove_unsafe_foreign_keys_for_codegen(database_url, &foreign_keys, &table_names);
    let table_data = table_names
        .into_iter()
        .map(|t| load_table_data(database_url, t, &config.column_sorting))
        .collect::<Result<Vec<_>, Box<dyn Error + Send + Sync + 'static>>>()?;

    let mut out = String::new();
    writeln!(out, "{}", SCHEMA_HEADER)?;

    let backend = Backend::for_url(database_url);

    let custom_types = if config.generate_missing_sql_type_definitions() {
        let diesel_provided_types = match backend {
            #[cfg(feature = "postgres")]
            Backend::Pg => pg_diesel_types(),
            #[cfg(feature = "sqlite")]
            Backend::Sqlite => sqlite_diesel_types(),
            #[cfg(feature = "mysql")]
            Backend::Mysql => mysql_diesel_types(),
        };

        let mut all_types = table_data
            .iter()
            .flat_map(|t| t.column_data.iter().map(|c| &c.ty))
            .filter(|t| !diesel_provided_types.contains(&t.rust_name as &str))
            .cloned()
            .collect::<Vec<_>>();

        all_types.sort_unstable_by(|a, b| a.rust_name.cmp(&b.rust_name));
        all_types.dedup_by(|a, b| a.rust_name.eq(&b.rust_name));
        all_types
    } else {
        Vec::new()
    };

    let definitions = TableDefinitions {
        tables: table_data,
        fk_constraints: foreign_keys,
        include_docs: config.with_docs,
        custom_type_defs: CustomTypeList {
            backend,
            types: custom_types,
            with_docs: config.with_docs,
        },
        import_types: config.import_types(),
    };

    if let Some(schema_name) = config.schema_name() {
        write!(out, "{}", ModuleDefinition(schema_name, definitions))?;
    } else {
        write!(out, "{}", definitions.custom_type_defs)?;
        write!(out, "{}", definitions)?;
    }

    if let Some(ref patch_file) = config.patch_file {
        let patch = std::fs::read_to_string(patch_file)?;
        let patch = diffy::Patch::from_str(&patch)?;

        out = diffy::apply(&out, &patch)?;
    }

    Ok(out)
}

struct CustomTypeList {
    backend: Backend,
    types: Vec<ColumnType>,
    with_docs: bool,
}

impl CustomTypeList {
    #[cfg(feature = "postgres")]
    fn contains(&self, tpe: &str) -> bool {
        self.types.iter().any(|c| c.rust_name == tpe)
    }
}

impl Display for CustomTypeList {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.types.is_empty() {
            return Ok(());
        }
        match self.backend {
            #[cfg(feature = "postgres")]
            Backend::Pg => {
                if self.with_docs {
                    writeln!(f, "/// A module containing custom SQL type definitions")?;
                    writeln!(f, "///")?;
                    writeln!(f, "/// (Automatically generated by Diesel.)")?;
                }
                let mut out = PadAdapter::new(f);
                writeln!(out, "pub mod sql_types {{")?;

                for (idx, t) in self.types.iter().enumerate() {
                    if idx != 0 {
                        writeln!(out)?;
                    }
                    if self.with_docs {
                        if let Some(ref schema) = t.schema {
                            writeln!(out, "/// The `{}.{}` SQL type", schema, t.sql_name)?;
                        } else {
                            writeln!(out, "/// The `{}` SQL type", t.sql_name)?;
                        }
                        writeln!(out, "///")?;
                        writeln!(out, "/// (Automatically generated by Diesel.)")?;
                    }
                    writeln!(out, "#[derive(diesel::sql_types::SqlType)]")?;
                    if let Some(ref schema) = t.schema {
                        writeln!(
                            out,
                            "#[postgres(type_name = \"{}\", type_schema = \"{}\")]",
                            t.sql_name, schema
                        )?;
                    } else {
                        writeln!(out, "#[postgres(type_name = \"{}\")]", t.sql_name)?;
                    }
                    writeln!(out, "pub struct {};", t.rust_name)?;
                }

                writeln!(f, "}}\n")?;
                Ok(())
            }
            #[cfg(feature = "sqlite")]
            Backend::Sqlite => {
                let _ = (&f, self.with_docs);
                for t in &self.types {
                    eprintln!("Encountered unknown type for Sqlite: {}", t.sql_name);
                }
                unreachable!(
                    "Diesel only support a closed set of types for Sqlite. \
                     If you ever see this error message please open an \
                     issue at https://github.com/diesel-rs/diesel containing \
                     a dump of your schema definition."
                )
            }
            #[cfg(feature = "mysql")]
            Backend::Mysql => {
                let _ = (&f, self.with_docs);
                for t in &self.types {
                    eprintln!("Encountered unknown type for Mysql: {}", t.sql_name);
                }
                unreachable!(
                    "Mysql only supports a closed set of types.
                         If you ever see this error message please open an \
                         issue at https://github.com/diesel-rs/diesel containing \
                         a dump of your schema definition."
                )
            }
        }
    }
}

struct ModuleDefinition<'a>(&'a str, TableDefinitions<'a>);

impl<'a> Display for ModuleDefinition<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        {
            let mut out = PadAdapter::new(f);
            writeln!(out, "pub mod {} {{", self.0)?;
            write!(out, "{}", self.1.custom_type_defs)?;
            write!(out, "{}", self.1)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

struct TableDefinitions<'a> {
    tables: Vec<TableData>,
    fk_constraints: Vec<ForeignKeyConstraint>,
    include_docs: bool,
    import_types: Option<&'a [String]>,
    custom_type_defs: CustomTypeList,
}

impl<'a> Display for TableDefinitions<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut is_first = true;
        for table in &self.tables {
            if is_first {
                is_first = false;
            } else {
                writeln!(f)?;
            }
            writeln!(
                f,
                "{}",
                TableDefinition {
                    table,
                    include_docs: self.include_docs,
                    import_types: self.import_types,
                    custom_type_defs: &self.custom_type_defs
                }
            )?;
        }

        if !self.fk_constraints.is_empty() {
            writeln!(f)?;
        }

        for foreign_key in &self.fk_constraints {
            writeln!(f, "{}", Joinable(foreign_key))?;
        }

        if self.tables.len() > 1 {
            write!(f, "\ndiesel::allow_tables_to_appear_in_same_query!(")?;
            {
                let mut out = PadAdapter::new(f);
                writeln!(out)?;
                for table in &self.tables {
                    if table.name.rust_name == table.name.sql_name {
                        writeln!(out, "{},", table.name.sql_name)?;
                    } else {
                        writeln!(out, "{},", table.name.rust_name)?;
                    }
                }
            }
            writeln!(f, ");")?;
        }

        Ok(())
    }
}

struct TableDefinition<'a> {
    table: &'a TableData,
    include_docs: bool,
    import_types: Option<&'a [String]>,
    custom_type_defs: &'a CustomTypeList,
}

impl<'a> Display for TableDefinition<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "diesel::table! {{")?;
        {
            let mut out = PadAdapter::new(f);
            writeln!(out)?;

            let mut has_written_import = false;
            if let Some(types) = self.import_types {
                for import in types {
                    writeln!(out, "use {};", import)?;
                    has_written_import = true;
                }
            }

            #[cfg(feature = "postgres")]
            for col in &self.table.column_data {
                if self.custom_type_defs.contains(&col.ty.rust_name) {
                    if !has_written_import {
                        writeln!(out, "use diesel::sql_types::*;")?;
                    }
                    writeln!(out, "use super::sql_types::{};", col.ty.rust_name)?;
                    has_written_import = true;
                }
            }
            #[cfg(not(feature = "postgres"))]
            let _ = self.custom_type_defs;

            if has_written_import {
                writeln!(out)?;
            }

            if self.include_docs {
                for d in self.table.docs.lines() {
                    writeln!(out, "///{}{}", if d.is_empty() { "" } else { " " }, d)?;
                }
            }

            if self.table.name.rust_name != self.table.name.sql_name {
                writeln!(
                    out,
                    r#"#[sql_name = "{}"]"#,
                    self.table.name.full_sql_name()
                )?;
            }

            write!(out, "{} (", self.table.name)?;

            for (i, pk) in self.table.primary_key.iter().enumerate() {
                if i != 0 {
                    write!(out, ", ")?;
                }
                write!(out, "{}", pk)?;
            }

            write!(
                out,
                ") {}",
                ColumnDefinitions {
                    columns: &self.table.column_data,
                    include_docs: self.include_docs,
                }
            )?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

struct ColumnDefinitions<'a> {
    columns: &'a [ColumnDefinition],
    include_docs: bool,
}

impl<'a> Display for ColumnDefinitions<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        {
            let mut out = PadAdapter::new(f);
            writeln!(out, "{{")?;
            for column in self.columns {
                if self.include_docs {
                    for d in column.docs.lines() {
                        writeln!(out, "///{}{}", if d.is_empty() { "" } else { " " }, d)?;
                    }
                }
                if column.rust_name == column.sql_name {
                    writeln!(out, "{} -> {},", column.sql_name, column.ty)?;
                } else {
                    writeln!(out, r#"#[sql_name = "{}"]"#, column.sql_name)?;
                    writeln!(out, "{} -> {},", column.rust_name, column.ty)?;
                }
            }
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

struct Joinable<'a>(&'a ForeignKeyConstraint);

impl<'a> Display for Joinable<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let child_table_name = &self.0.child_table.rust_name;

        let parent_table_name = &self.0.parent_table.rust_name;

        write!(
            f,
            "diesel::joinable!({} -> {} ({}));",
            child_table_name, parent_table_name, self.0.foreign_key_rust_name,
        )
    }
}

/// Lifted directly from libcore/fmt/builders.rs
struct PadAdapter<'a, 'b: 'a> {
    fmt: &'a mut Formatter<'b>,
    on_newline: bool,
}

impl<'a, 'b: 'a> PadAdapter<'a, 'b> {
    fn new(fmt: &'a mut Formatter<'b>) -> PadAdapter<'a, 'b> {
        PadAdapter {
            fmt,
            on_newline: false,
        }
    }
}

impl<'a, 'b: 'a> Write for PadAdapter<'a, 'b> {
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        while !s.is_empty() {
            let on_newline = self.on_newline;

            let split = match s.find('\n') {
                Some(pos) => {
                    self.on_newline = true;
                    pos + 1
                }
                None => {
                    self.on_newline = false;
                    s.len()
                }
            };

            let to_write = &s[..split];
            if on_newline && to_write != "\n" {
                self.fmt.write_str("    ")?;
            }
            self.fmt.write_str(to_write)?;

            s = &s[split..];
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for Filtering {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FilteringVisitor;

        impl<'de> Visitor<'de> for FilteringVisitor {
            type Value = Filtering;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("either only_tables or except_tables")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut only_tables = None::<Vec<Regex>>;
                let mut except_tables = None::<Vec<Regex>>;
                while let Some(key) = map.next_key()? {
                    match key {
                        "only_tables" => {
                            if only_tables.is_some() {
                                return Err(de::Error::duplicate_field("only_tables"));
                            }
                            only_tables = Some(map.next_value()?);
                        }
                        "except_tables" => {
                            if except_tables.is_some() {
                                return Err(de::Error::duplicate_field("except_tables"));
                            }
                            except_tables = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                key,
                                &["only_tables", "except_tables"],
                            ))
                        }
                    }
                }
                match (only_tables, except_tables) {
                    (Some(t), None) => Ok(Filtering::OnlyTables(t)),
                    (None, Some(t)) => Ok(Filtering::ExceptTables(t)),
                    (None, None) => Ok(Filtering::None),
                    _ => Err(de::Error::duplicate_field("only_tables except_tables")),
                }
            }
        }

        deserializer.deserialize_map(FilteringVisitor)
    }
}
