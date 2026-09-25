//! View nullability inference checked against SQLite.
//!
//! `diesel print-schema --experimental-infer-nullable-for-views` declares a view
//! column NOT NULL when `diesel_infer_query` infers that it cannot be NULL, so the
//! view must never return NULL there. Each input creates tables, rows, and a view in
//! a fresh in-memory SQLite database, infers the nullability of the view's columns
//! from the same `CREATE VIEW` text print-schema reads, and checks the number of the
//! view's columns and every row SQLite returns against it. Like print-schema, which
//! only reads views the database accepted, it only infers views SQLite accepts.

use arbitrary::Unstructured;
use diesel::connection::{LoadConnection, SimpleConnection};
use diesel::prelude::*;
use diesel::row::{Field, Row};
use diesel::sqlite::{AuthorizerContext, AuthorizerDecision, ProgressDecision, authorizer};
use diesel_infer_query::{IsNull, SchemaField, SchemaResolver};
use std::error::Error;
use std::fmt::Write;
use std::num::NonZeroU32;
use std::sync::LazyLock;

/// SQLite virtual machine steps between two progress callbacks.
const PROGRESS_INTERVAL: NonZeroU32 = NonZeroU32::new(1_000).unwrap();
/// Progress callbacks before a query is interrupted, which bounds raw SQL like
/// recursive CTEs.
const PROGRESS_BUDGET: u32 = 1_000;
/// Nesting depth from which generated expressions are only columns or literals.
const MAX_DEPTH: usize = 4;

const TABLE_NAMES: [&str; 3] = ["ta", "tb", "tc"];
const ALIASES: [&str; 3] = ["sa", "sb", "sc"];
const COLUMN_NAMES: [&str; 3] = ["ca", "cb", "cc"];

const BINARY_OPERATORS: [&str; 18] = [
    " + ",
    " - ",
    " * ",
    " / ",
    " % ",
    " || ",
    " = ",
    " <> ",
    " < ",
    " > ",
    " AND ",
    " OR ",
    " & ",
    " << ",
    " IS DISTINCT FROM ",
    " IS NOT DISTINCT FROM ",
    " -> ",
    " ->> ",
];

/// Functions and their number of arguments, which turns `max` and `min` into scalar
/// functions with two.
const FUNCTIONS: [(&str, usize); 15] = [
    ("abs", 1),
    ("lower", 1),
    ("length", 1),
    ("coalesce", 2),
    ("nullif", 2),
    ("ifnull", 2),
    ("max", 1),
    ("max", 2),
    ("min", 1),
    ("min", 2),
    ("sum", 1),
    ("total", 1),
    ("avg", 1),
    ("count", 1),
    ("group_concat", 1),
];

/// A table with its rows.
#[derive(Debug)]
pub struct Table {
    name: String,
    columns: Vec<Column>,
    rows: Vec<Vec<Value>>,
}

#[derive(Debug)]
struct Column {
    name: String,
    text: bool,
    nullable: bool,
}

/// A value stored in a table, or used as a literal.
#[derive(Clone, Copy, Debug)]
enum Value {
    Null,
    Zero,
    One,
    MinusOne,
    EmptyText,
    Key,
    EmptyObject,
    ObjectWithKey,
}

const VALUES: [Value; 8] = [
    Value::Null,
    Value::Zero,
    Value::One,
    Value::MinusOne,
    Value::EmptyText,
    Value::Key,
    Value::EmptyObject,
    Value::ObjectWithKey,
];

impl Value {
    fn sql(self) -> &'static str {
        match self {
            Value::Null => "NULL",
            Value::Zero => "0",
            Value::One => "1",
            Value::MinusOne => "-1",
            Value::EmptyText => "''",
            Value::Key => "'k'",
            Value::EmptyObject => "'{}'",
            Value::ObjectWithKey => "'{\"k\": 1}'",
        }
    }
}

/// The tables the raw SQL lane queries, named like those of diesel_infer_query's tests.
///
/// One user has no posts and there are no comments, and the texts include valid JSON
/// with and without the key `k`, so outer joins, aggregates of empty tables, division
/// by zero, and JSON extraction all produce NULL.
static FIXTURE: LazyLock<Vec<Table>> = LazyLock::new(|| {
    use Value::*;
    vec![
        table(
            "users",
            &[("id", false), ("name", false), ("hair_color", true)],
            vec![vec![Zero, EmptyObject, Null], vec![One, ObjectWithKey, Key]],
        ),
        table(
            "posts",
            &[
                ("id", false),
                ("user_id", false),
                ("title", false),
                ("body", true),
            ],
            vec![vec![Zero, One, EmptyText, Null]],
        ),
        table(
            "comments",
            &[("id", false), ("post_id", false), ("body", true)],
            vec![],
        ),
    ]
});

/// A fixture table whose columns named `id` or ending in `_id` are integers.
fn table(name: &str, columns: &[(&str, bool)], rows: Vec<Vec<Value>>) -> Table {
    Table {
        name: name.to_owned(),
        columns: columns
            .iter()
            .map(|&(name, nullable)| Column {
                name: name.to_owned(),
                text: name != "id" && !name.ends_with("_id"),
                nullable,
            })
            .collect(),
        rows,
    }
}

/// Run one fuzzer input: an even first byte selects the rest as raw SQL over the
/// fixture tables, an odd one generated tables and a view.
pub fn run_case(data: &[u8]) -> Result<(), String> {
    let Some((&lane, rest)) = data.split_first() else {
        return Ok(());
    };
    if lane % 2 == 0 {
        match std::str::from_utf8(rest) {
            Ok(select) => check_raw_sql(select),
            Err(_) => Ok(()),
        }
    } else {
        match generate(&mut Unstructured::new(rest)) {
            Ok((tables, select)) => check(&tables, &select),
            Err(_) => Ok(()),
        }
    }
}

/// Check the view `select` over the fixture tables.
pub fn check_raw_sql(select: &str) -> Result<(), String> {
    check(&FIXTURE, select)
}

/// Check the nullability claims in `inferred` for the view `select` over the fixture
/// tables, which lets tests confirm that a wrong claim is caught.
pub fn check_fixture_claims(select: &str, inferred: &[IsNull]) -> Result<(), String> {
    let schema = schema_sql(&FIXTURE);
    match create_view(&schema, &format!("CREATE VIEW v AS {select}"))? {
        Some((mut conn, definition)) => {
            check_rows(&mut conn, inferred).map_err(|e| format!("{e}\n{schema}{definition}"))
        }
        None => Ok(()),
    }
}

/// Infer the nullability of `CREATE VIEW v AS {select}` and check it against the rows
/// SQLite returns for the view.
fn check(tables: &[Table], select: &str) -> Result<(), String> {
    let schema = schema_sql(tables);
    // print-schema only reads views the database accepted, and this also keeps input
    // that is not SQL at all, which sqlparser can take exponential time to reject, away
    // from the parser
    let Some((mut conn, definition)) = create_view(&schema, &format!("CREATE VIEW v AS {select}"))?
    else {
        return Ok(());
    };
    // parsing and inference must never panic, but they may reject any view
    let Ok(inferred) = infer(tables, &definition) else {
        return Ok(());
    };
    check_rows(&mut conn, &inferred).map_err(|e| format!("{e}\n{schema}{definition}"))
}

/// The inferred nullability of the view's columns.
fn infer(tables: &[Table], definition: &str) -> diesel_infer_query::Result<Vec<IsNull>> {
    let mut schema = Schema(tables);
    let mut view =
        diesel_infer_query::parse_view_def(definition, diesel_infer_query::Backend::Sqlite)?;
    view.resolve_references(&mut schema)?;
    view.infer_nullability(&mut schema)
}

/// A fresh in-memory database with the tables `schema` creates and the view
/// `definition`, and the definition SQLite stored for the view, or `None` if SQLite
/// rejects the view.
fn create_view(
    schema: &str,
    definition: &str,
) -> Result<Option<(SqliteConnection, String)>, String> {
    let mut conn = SqliteConnection::establish(":memory:")
        .map_err(|e| format!("failed to open an in-memory database: {e}"))?;
    conn.set_recommended_security_limits();
    // their results differ between runs of the same input; the current time does
    // too, but denying the date and time functions would lose their NULL results
    conn.on_authorize(|action| match action {
        AuthorizerContext::Function(authorizer::Function {
            function: Some(name),
            ..
        }) if name.eq_ignore_ascii_case("random") || name.eq_ignore_ascii_case("randomblob") => {
            AuthorizerDecision::Deny
        }
        _ => AuthorizerDecision::Allow,
    });
    let mut callbacks = 0;
    conn.on_progress(PROGRESS_INTERVAL, move || {
        callbacks += 1;
        if callbacks > PROGRESS_BUDGET {
            ProgressDecision::Interrupt
        } else {
            ProgressDecision::Continue
        }
    });
    conn.batch_execute(schema)
        .map_err(|e| format!("failed to create the tables: {e}\n{schema}"))?;
    if diesel::sql_query(definition).execute(&mut conn).is_err() {
        return Ok(None);
    }
    // SQLite runs and stores only the first statement, and print-schema reads what it
    // stored, so whatever follows never reaches the parser either
    let Ok(stored) = diesel::sql_query("SELECT sql FROM sqlite_schema WHERE name = 'v'")
        .get_result::<StoredView>(&mut conn)
    else {
        return Ok(None);
    };
    Ok(Some((conn, stored.sql)))
}

#[derive(QueryableByName)]
struct StoredView {
    #[diesel(sql_type = diesel::sql_types::Text)]
    sql: String,
}

#[derive(QueryableByName)]
struct ViewColumns {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    columns: i64,
}

/// Check that the view has as many columns as `inferred` has claims, and that no row of
/// it has NULL in a column `inferred` calls NOT NULL.
fn check_rows(conn: &mut SqliteConnection, inferred: &[IsNull]) -> Result<(), String> {
    // print-schema pairs the claims with the view's columns by position, and the rows
    // alone would not show a wrong number of claims for a view without rows
    let Ok(view) = diesel::sql_query("SELECT count(*) AS columns FROM pragma_table_xinfo('v')")
        .get_result::<ViewColumns>(conn)
    else {
        return Ok(());
    };
    if usize::try_from(view.columns) != Ok(inferred.len()) {
        return Err(format!(
            "inferred {} columns, but SQLite's view has {}",
            inferred.len(),
            view.columns
        ));
    }
    if !inferred.contains(&IsNull::NotNullable) {
        return Ok(());
    }
    let Ok(rows) = conn.load(diesel::sql_query("SELECT * FROM v")) else {
        return Ok(());
    };
    for row in rows {
        // a row can fail to compute, for example extracting from malformed JSON
        let Ok(row) = row else {
            return Ok(());
        };
        for (index, nullable) in inferred.iter().enumerate() {
            if *nullable == IsNull::NotNullable
                && row.get(index).is_some_and(|field| field.is_null())
            {
                return Err(format!(
                    "column {index} was inferred NOT NULL, but SQLite returned NULL for it"
                ));
            }
        }
    }
    Ok(())
}

fn schema_sql(tables: &[Table]) -> String {
    let mut sql = String::new();
    for table in tables {
        let columns = table
            .columns
            .iter()
            .map(|c| {
                let ty = if c.text { "TEXT" } else { "INTEGER" };
                let constraint = if c.nullable { "" } else { " NOT NULL" };
                format!("{} {ty}{constraint}", c.name)
            })
            .collect::<Vec<_>>();
        writeln!(sql, "CREATE TABLE {} ({});", table.name, columns.join(", "))
            .expect("writing to a string");
        for row in &table.rows {
            let values = row
                .iter()
                .zip(&table.columns)
                .map(|(value, column)| match value {
                    Value::Null if !column.nullable => "0",
                    value => value.sql(),
                })
                .collect::<Vec<_>>();
            writeln!(
                sql,
                "INSERT INTO {} VALUES ({});",
                table.name,
                values.join(", ")
            )
            .expect("writing to a string");
        }
    }
    sql
}

/// Resolves names like SQLite, ignoring ASCII case and accepting the `main` schema.
struct Schema<'t>(&'t [Table]);

impl Schema<'_> {
    fn table(
        &self,
        schema: Option<&str>,
        relation: &str,
    ) -> Result<&Table, Box<dyn Error + Send + Sync + 'static>> {
        if schema.is_some_and(|schema| !schema.eq_ignore_ascii_case("main")) {
            return Err(format!("no schema `{schema:?}`").into());
        }
        self.0
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(relation))
            .ok_or_else(|| format!("no table `{relation}`").into())
    }
}

impl SchemaResolver for Schema<'_> {
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
        field_name: &str,
    ) -> Result<&'s dyn SchemaField, Box<dyn Error + Send + Sync + 'static>> {
        let relation = query_relation.ok_or("no table for an unnamed query source")?;
        self.table(relation_schema, relation)?
            .columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(field_name))
            .map(|c| c as &dyn SchemaField)
            .ok_or_else(|| format!("no column `{relation}.{field_name}`").into())
    }

    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn Error + Send + Sync + 'static>> {
        let relation = query_relation.ok_or("no table for an unnamed query source")?;
        Ok(self
            .table(relation_schema, relation)?
            .columns
            .iter()
            .map(|c| c as &dyn SchemaField)
            .collect())
    }
}

impl SchemaField for Column {
    fn is_nullable(&self) -> IsNull {
        if self.nullable {
            IsNull::IsNullable
        } else {
            IsNull::NotNullable
        }
    }

    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }
}

/// Generate up to three tables with rows, and a view selecting from them.
///
/// Every decision takes a single byte, and running out of input picks the simplest
/// option, so short inputs already reach joins, `CASE`, operators, and aggregates.
fn generate(u: &mut Unstructured<'_>) -> arbitrary::Result<(Vec<Table>, String)> {
    let tables = TABLE_NAMES
        .iter()
        .take(u.int_in_range(1..=3)?)
        .map(|name| generate_table(u, name))
        .collect::<arbitrary::Result<Vec<_>>>()?;
    let scope = Scope::generate(u, &tables)?;
    let mut select = String::new();
    scope.select(u, &mut select)?;
    Ok((tables, select))
}

fn generate_table(u: &mut Unstructured<'_>, name: &str) -> arbitrary::Result<Table> {
    let columns = COLUMN_NAMES
        .iter()
        .take(u.int_in_range(1..=3)?)
        .map(|name| {
            let flags = u.int_in_range(0..=3)?;
            Ok(Column {
                name: (*name).to_owned(),
                text: flags & 1 != 0,
                nullable: flags & 2 != 0,
            })
        })
        .collect::<arbitrary::Result<Vec<_>>>()?;
    let rows = (0..u.int_in_range(0..=3)?)
        .map(|_| columns.iter().map(|_| u.choose(&VALUES).copied()).collect())
        .collect::<arbitrary::Result<_>>()?;
    Ok(Table {
        name: name.to_owned(),
        columns,
        rows,
    })
}

/// SQLite matches names ignoring ASCII case, so vary it.
#[derive(Clone, Copy, Debug)]
enum Spelling {
    Lower,
    Upper,
    Capitalized,
}

impl Spelling {
    fn generate(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        u.choose(&[Spelling::Lower, Spelling::Upper, Spelling::Capitalized])
            .copied()
    }

    fn push(self, name: &str, out: &mut String) {
        match self {
            Spelling::Lower => out.push_str(name),
            Spelling::Upper => out.push_str(&name.to_ascii_uppercase()),
            Spelling::Capitalized => {
                let mut chars = name.chars();
                if let Some(first) = chars.next() {
                    out.push(first.to_ascii_uppercase());
                    out.push_str(chars.as_str());
                }
            }
        }
    }
}

/// A table in the generated FROM clause, and the name the view refers to it by.
struct Source<'t> {
    name: &'static str,
    table: &'t Table,
    spelling: Spelling,
}

/// The sources of the generated FROM clause.
struct Scope<'t> {
    sources: Vec<Source<'t>>,
}

impl<'t> Scope<'t> {
    fn generate(u: &mut Unstructured<'_>, tables: &'t [Table]) -> arbitrary::Result<Self> {
        let mut sources = Vec::<Source<'t>>::new();
        for alias in ALIASES.iter().take(u.int_in_range(1..=3)?) {
            let index = u.choose_index(tables.len())?;
            // a table joined twice under the same name makes every reference ambiguous
            let taken = sources
                .iter()
                .any(|source| source.name == TABLE_NAMES[index]);
            let aliased = u.arbitrary::<bool>()? || taken;
            sources.push(Source {
                name: if aliased { alias } else { TABLE_NAMES[index] },
                table: &tables[index],
                spelling: Spelling::generate(u)?,
            });
        }
        Ok(Self { sources })
    }

    fn select(&self, u: &mut Unstructured<'_>, sql: &mut String) -> arbitrary::Result<()> {
        sql.push_str("SELECT ");
        for item in 0..u.int_in_range(1..=4)? {
            if item > 0 {
                sql.push_str(", ");
            }
            match u.int_in_range(0..=5)? {
                0 => sql.push('*'),
                1 => {
                    let source = u.choose(&self.sources)?;
                    Spelling::generate(u)?.push(source.name, sql);
                    sql.push_str(".*");
                }
                _ => self.expr(u, 0, sql)?,
            }
        }
        sql.push_str(" FROM ");
        for (index, source) in self.sources.iter().enumerate() {
            if index > 0 {
                let left = u.arbitrary::<bool>()?;
                sql.push_str(if left { " LEFT JOIN " } else { " JOIN " });
            }
            source.spelling.push(&source.table.name, sql);
            if source.name != source.table.name {
                write!(sql, " AS {}", source.name).expect("writing to a string");
            }
            if index > 0 {
                // any other source, so joins can also refer to later ones and to each other
                sql.push_str(" ON ");
                self.column_of(u, source, true, sql)?;
                sql.push_str(" = ");
                let other = u.choose_index(self.sources.len() - 1)?;
                let other = &self.sources[if other < index { other } else { other + 1 }];
                self.column_of(u, other, true, sql)?;
            }
        }
        // bit 0 adds a WHERE, bit 1 a GROUP BY, and bit 2 a HAVING clause
        let clauses = u.int_in_range(0..=7)?;
        if clauses & 1 != 0 {
            sql.push_str(" WHERE ");
            self.expr(u, 0, sql)?;
        }
        if clauses & 2 != 0 {
            sql.push_str(" GROUP BY ");
            self.column(u, sql)?;
        }
        if clauses & 4 != 0 {
            sql.push_str(" HAVING ");
            self.expr(u, 0, sql)?;
        }
        Ok(())
    }

    fn column(&self, u: &mut Unstructured<'_>, sql: &mut String) -> arbitrary::Result<()> {
        let source = u.choose(&self.sources)?;
        let qualified = u.arbitrary::<bool>()?;
        self.column_of(u, source, qualified, sql)
    }

    fn column_of(
        &self,
        u: &mut Unstructured<'_>,
        source: &Source<'_>,
        qualified: bool,
        sql: &mut String,
    ) -> arbitrary::Result<()> {
        let spelling = Spelling::generate(u)?;
        // with several sources an unqualified name can be ambiguous
        if qualified || self.sources.len() > 1 {
            spelling.push(source.name, sql);
            sql.push('.');
        }
        spelling.push(&u.choose(&source.table.columns)?.name, sql);
        Ok(())
    }

    fn expr(
        &self,
        u: &mut Unstructured<'_>,
        depth: usize,
        sql: &mut String,
    ) -> arbitrary::Result<()> {
        let kinds = if depth < MAX_DEPTH { 11 } else { 1 };
        let depth = depth + 1;
        match u.int_in_range(0..=kinds)? {
            0 => self.column(u, sql)?,
            1 => sql.push_str(u.choose(&VALUES)?.sql()),
            2 => {
                self.expr(u, depth, sql)?;
                sql.push_str(u.choose(&BINARY_OPERATORS)?);
                self.expr(u, depth, sql)?;
            }
            3 => {
                self.expr(u, depth, sql)?;
                let negated = u.arbitrary::<bool>()?;
                sql.push_str(if negated { " IS NOT NULL" } else { " IS NULL" });
            }
            4 => {
                self.expr(u, depth, sql)?;
                // the bounds are leaves, so no operator in them binds to the `AND`
                sql.push_str(" BETWEEN ");
                self.expr(u, MAX_DEPTH, sql)?;
                sql.push_str(" AND ");
                self.expr(u, MAX_DEPTH, sql)?;
            }
            5 => {
                self.expr(u, depth, sql)?;
                sql.push_str(" LIKE ");
                self.expr(u, MAX_DEPTH, sql)?;
            }
            6 => self.case(u, depth, sql)?,
            7 => {
                sql.push_str("CAST(");
                self.expr(u, depth, sql)?;
                let text = u.arbitrary::<bool>()?;
                sql.push_str(if text { " AS TEXT)" } else { " AS INTEGER)" });
            }
            8 => {
                let (name, arguments) = *u.choose(&FUNCTIONS)?;
                sql.push_str(name);
                sql.push('(');
                for argument in 0..arguments {
                    if argument > 0 {
                        sql.push_str(", ");
                    }
                    self.expr(u, depth, sql)?;
                }
                sql.push(')');
            }
            9 => sql.push_str("count(*)"),
            10 => sql.push_str("count(*) OVER ()"),
            _ => {
                sql.push('(');
                self.expr(u, depth, sql)?;
                sql.push(')');
            }
        }
        Ok(())
    }

    fn case(
        &self,
        u: &mut Unstructured<'_>,
        depth: usize,
        sql: &mut String,
    ) -> arbitrary::Result<()> {
        sql.push_str("CASE ");
        if u.arbitrary::<bool>()? {
            self.expr(u, depth, sql)?;
            sql.push(' ');
        }
        for _ in 0..u.int_in_range(1..=2)? {
            sql.push_str("WHEN ");
            self.expr(u, depth, sql)?;
            sql.push_str(" THEN ");
            self.expr(u, depth, sql)?;
            sql.push(' ');
        }
        if u.arbitrary::<bool>()? {
            sql.push_str("ELSE ");
            self.expr(u, depth, sql)?;
            sql.push(' ');
        }
        sql.push_str("END");
        Ok(())
    }
}
