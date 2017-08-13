use diesel_infer_schema::*;
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};

pub enum Filtering {
    Whitelist(Vec<TableName>),
    Blacklist(Vec<TableName>),
    None,
}

impl Filtering {
    pub fn should_ignore_table(&self, name: &TableName) -> bool {
        use self::Filtering::*;

        match *self {
            Whitelist(ref names) => !names.contains(name),
            Blacklist(ref names) => names.contains(name),
            None => false,
        }
    }
}

pub fn run_print_schema(
    database_url: &str,
    schema_name: Option<&str>,
    filtering: &Filtering
) -> Result<(), Box<Error>> {
    let table_data = load_table_names(database_url, schema_name)?
        .into_iter()
        .filter(|t| !filtering.should_ignore_table(t))
        .map(|t| load_table_data(database_url, t))
        .collect::<Result<_, Box<Error>>>()?;
    let definitions = TableDefinitions(table_data);

    if let Some(schema_name) = schema_name {
        print!("{}", ModuleDefinition(schema_name, definitions));
    } else {
        print!("{}", definitions);
    }
    Ok(())
}

struct ModuleDefinition<'a>(&'a str, TableDefinitions);

impl<'a> Display for ModuleDefinition<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        {
            let mut out = PadAdapter::new(f);
            writeln!(out, "pub mod {} {{", self.0)?;
            write!(out, "{}", self.1)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

struct TableDefinitions(Vec<TableData>);

impl Display for TableDefinitions {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut is_first = true;
        for table in &self.0 {
            if is_first {
                is_first = false;
            } else {
                write!(f, "\n")?;
            }
            writeln!(f, "{}", TableDefinition(table))?;
        }
        Ok(())
    }
}

struct TableDefinition<'a>(&'a TableData);

impl<'a> Display for TableDefinition<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "table! {{")?;
        {
            let mut out = PadAdapter::new(f);
            write!(out, "\n{} (", self.0.name)?;
            for (i, pk) in self.0.primary_key.iter().enumerate() {
                if i != 0 {
                    write!(out, ", ")?;
                }
                write!(out, "{}", pk)?;
            }
            write!(out, ") {}", ColumnDefinitions(&self.0.column_data))?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

struct ColumnDefinitions<'a>(&'a [ColumnDefinition]);

impl<'a> Display for ColumnDefinitions<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        {
            let mut out = PadAdapter::new(f);
            writeln!(out, "{{")?;
            for column in self.0 {
                write!(out, "{} -> ", column.name)?;
                if column.ty.is_nullable {
                    write!(out, "Nullable<")?;
                }
                if column.ty.is_array {
                    write!(out, "Array<")?;
                }
                write!(out, "{}", column.ty.rust_name)?;
                if column.ty.is_array {
                    write!(out, ">")?;
                }
                if column.ty.is_nullable {
                    write!(out, ">")?;
                }
                writeln!(out, ",")?;
            }
        }
        writeln!(f, "}}")?;
        Ok(())
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
            fmt: fmt,
            on_newline: false,
        }
    }
}

impl<'a, 'b: 'a> Write for PadAdapter<'a, 'b> {
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        while !s.is_empty() {
            if self.on_newline {
                self.fmt.write_str("    ")?;
            }

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
            self.fmt.write_str(&s[..split])?;
            s = &s[split..];
        }

        Ok(())
    }
}
