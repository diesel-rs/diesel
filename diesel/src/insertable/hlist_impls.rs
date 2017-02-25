use hlist::*;
use super::*;
use query_builder::QueryFragment;

impl<Head, Tail, DB> InsertValues<DB> for Cons<Head, Tail> where
    DB: Backend,
    Cons<Head, Tail>: InsertValuesRecursive<DB>,
{
    fn column_names(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        InsertValuesRecursive::<DB>::column_names(self, false, out)
    }

    fn values_clause(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("(");
        try!(InsertValuesRecursive::<DB>::values_clause(self, false, out));
        out.push_sql(")");
        Ok(())
    }

    fn values_bind_params(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        InsertValuesRecursive::<DB>::values_bind_params(self, out)
    }
}

#[doc(hidden)]
pub trait InsertValuesRecursive<DB: Backend> {
    fn column_names(&self, comma_needed: bool, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn values_clause(&self, comma_needed: bool, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn values_bind_params(&self, out: &mut DB::BindCollector) -> QueryResult<()>;
}

impl<Col, Expr, Tail, DB> InsertValuesRecursive<DB>
    for Cons<ColumnInsertValue<Col, Expr>, Tail> where
        DB: Backend + SupportsDefaultKeyword,
        Col: Column,
        Col::SqlType: IntoNullable,
        Expr: Expression<SqlType=<Col::SqlType as IntoNullable>::Nullable> + QueryFragment<DB>,
        Tail: InsertValuesRecursive<DB>,
{
    fn column_names(&self, comma_needed: bool, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        if comma_needed {
            out.push_sql(", ");
        }
        try!(out.push_identifier(Col::name()));
        self.1.column_names(true, out)
    }

    fn values_clause(&self, comma_needed: bool, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        if comma_needed {
            out.push_sql(", ");
        }
        match self.0 {
            ColumnInsertValue::Expression(_, ref value) => {
                try!(value.to_sql(out));
            }
            _ => out.push_sql("DEFAULT"),
        }
        self.1.values_clause(true, out)
    }

    fn values_bind_params(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(_, ref value) = self.0 {
            try!(value.collect_binds(out));
        }
        self.1.values_bind_params(out)
    }
}

#[cfg(feature="sqlite")]
use sqlite::Sqlite;

#[cfg(feature="sqlite")]
impl<Col, Expr, Tail> InsertValuesRecursive<Sqlite>
    for Cons<ColumnInsertValue<Col, Expr>, Tail> where
        Col: Column,
        Col::SqlType: IntoNullable,
        Expr: Expression<SqlType=<Col::SqlType as IntoNullable>::Nullable> + QueryFragment<Sqlite>,
        Tail: InsertValuesRecursive<Sqlite>,
{
    fn column_names(
        &self,
        mut comma_needed: bool,
        out: &mut <Sqlite as Backend>::QueryBuilder,
    ) -> BuildQueryResult {
        if let ColumnInsertValue::Expression(..) = self.0 {
            if comma_needed {
                out.push_sql(", ");
            }
            try!(out.push_identifier(Col::name()));
            comma_needed = true;
        }
        self.1.column_names(comma_needed, out)
    }

    fn values_clause(
        &self,
        mut comma_needed: bool,
        out: &mut <Sqlite as Backend>::QueryBuilder,
    ) -> BuildQueryResult {
        if let ColumnInsertValue::Expression(_, ref value) = self.0 {
            if comma_needed {
                out.push_sql(", ");
            }
            try!(value.to_sql(out));
            comma_needed = true;
        }
        self.1.values_clause(comma_needed, out)
    }

    fn values_bind_params(
        &self,
        out: &mut <Sqlite as Backend>::BindCollector,
    ) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(_, ref value) = self.0 {
            try!(value.collect_binds(out));
        }
        self.1.values_bind_params(out)
    }
}

impl<DB: Backend> InsertValuesRecursive<DB> for Nil {
    fn column_names(&self, _: bool, _: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }

    fn values_clause(&self, _: bool, _: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }

    fn values_bind_params(&self, _: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }
}
