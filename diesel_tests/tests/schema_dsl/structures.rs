pub struct CreateTable<'a, Cols> {
    name: &'a str,
    columns: Cols
}

impl<'a, Cols> CreateTable<'a, Cols> {
    pub fn new(name: &'a str, columns: Cols) -> Self {
        CreateTable {
            name: name,
            columns: columns,
        }
    }
}

pub struct Column<'a, T> {
    name: &'a str,
    type_name: &'a str,
    _tpe: T,
}

impl<'a, T> Column<'a, T> {
    pub fn new(name: &'a str, type_name: &'a str, tpe: T) -> Self {
        Column {
            name: name,
            type_name: type_name,
            _tpe: tpe,
        }
    }

    pub fn primary_key(self) -> PrimaryKey<Self> {
        PrimaryKey(self)
    }

    pub fn not_null(self) -> NotNull<Self> {
        NotNull(self)
    }
}

pub struct PrimaryKey<Col>(Col);

impl<Col> PrimaryKey<Col> {
    pub fn auto_increment(self) -> AutoIncrement<Self> {
        AutoIncrement(self)
    }
}

pub struct AutoIncrement<Col>(Col);

pub struct NotNull<Col>(Col);

impl<'a, T> NotNull<Column<'a, T>> {
    pub fn default<'b>(self, expr: &'b str) -> Default<'b, Self> {
        Default {
            column: self,
            value: expr,
        }
    }
}

pub struct Default<'a, Col> {
    column: Col,
    value: &'a str,
}

use diesel::backend::*;
use diesel::query_builder::*;
use diesel::types::Integer;

impl<'a, DB, Cols> QueryFragment<DB> for CreateTable<'a, Cols> where
    DB: Backend,
    Cols: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("CREATE TABLE ");
        try!(out.push_identifier(self.name));
        out.push_sql(" (");
        try!(self.columns.to_sql(out));
        out.push_sql(")");
        Ok(())
    }
}

impl<'a, DB, T> QueryFragment<DB> for Column<'a, T> where
    DB: Backend,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(out.push_identifier(self.name));
        out.push_sql(" ");
        out.push_sql(self.type_name);
        Ok(())
    }
}

impl<DB, Col> QueryFragment<DB> for PrimaryKey<Col> where
    DB: Backend,
    Col: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.0.to_sql(out));
        out.push_sql(" PRIMARY KEY");
        Ok(())
    }
}

impl<Col> QueryFragment<Sqlite> for AutoIncrement<Col> where
    Col: QueryFragment<Sqlite>,
{
    fn to_sql(&self, out: &mut <Sqlite as Backend>::QueryBuilder) -> BuildQueryResult {
        try!(self.0.to_sql(out));
        out.push_sql(" AUTOINCREMENT");
        Ok(())
    }
}

impl<'a> QueryFragment<Pg> for AutoIncrement<PrimaryKey<Column<'a, Integer>>> {
    fn to_sql(&self, out: &mut <Pg as Backend>::QueryBuilder) -> BuildQueryResult {
        try!(out.push_identifier((self.0).0.name));
        out.push_sql(" SERIAL PRIMARY KEY");
        Ok(())
    }
}

impl<DB, Col> QueryFragment<DB> for NotNull<Col> where
    DB: Backend,
    Col: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.0.to_sql(out));
        out.push_sql(" NOT NULL");
        Ok(())
    }
}

impl<'a, DB, Col> QueryFragment<DB> for Default<'a, Col> where
    DB: Backend,
    Col: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.column.to_sql(out));
        out.push_sql(" DEFAULT ");
        out.push_sql(self.value);
        Ok(())
    }
}
