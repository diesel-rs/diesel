use crate::query_builder::insert_statement::{BatchInsert, InsertFromSelect};
#[cfg(feature = "sqlite")]
use crate::query_builder::where_clause::{BoxedWhereClause, WhereClause};
#[cfg(any(feature = "sqlite", feature = "postgres"))]
use crate::query_builder::{AstPass, QueryFragment};
use crate::query_builder::{BoxedSelectStatement, Query, SelectStatement, ValuesClause};
#[cfg(any(feature = "sqlite", feature = "postgres"))]
use crate::result::QueryResult;

pub trait IntoConflictValueClause {
    type ValueClause;

    fn into_value_clause(self) -> Self::ValueClause;
}

#[derive(Debug, Clone, Copy)]
pub struct OnConflictSelectWrapper<S>(S);

impl<Q> Query for OnConflictSelectWrapper<Q>
where
    Q: Query,
{
    type SqlType = Q::SqlType;
}

#[cfg(feature = "postgres")]
impl<S> QueryFragment<crate::pg::Pg> for OnConflictSelectWrapper<S>
where
    S: QueryFragment<crate::pg::Pg>,
{
    fn walk_ast<'a, 'b>(&'a self, out: AstPass<'_, 'b, crate::pg::Pg>) -> QueryResult<()>
    where
        'a: 'b,
    {
        self.0.walk_ast(out)
    }
}

// The corresponding impl for`NoWhereClause` is missing because of
// https://www.sqlite.org/lang_UPSERT.html (Parsing Ambiguity)
#[cfg(feature = "sqlite")]
impl<F, S, D, W, O, LOf, G, H, LC> QueryFragment<crate::sqlite::Sqlite>
    for OnConflictSelectWrapper<SelectStatement<F, S, D, WhereClause<W>, O, LOf, G, H, LC>>
where
    SelectStatement<F, S, D, WhereClause<W>, O, LOf, G, H, LC>:
        QueryFragment<crate::sqlite::Sqlite>,
{
    fn walk_ast<'a, 'b>(&'a self, out: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()>
    where
        'a: 'b,
    {
        self.0.walk_ast(out)
    }
}

#[cfg(feature = "sqlite")]
impl<'a, ST, QS, GB> QueryFragment<crate::sqlite::Sqlite>
    for OnConflictSelectWrapper<BoxedSelectStatement<'a, ST, QS, crate::sqlite::Sqlite, GB>>
where
    BoxedSelectStatement<'a, ST, QS, crate::sqlite::Sqlite, GB>:
        QueryFragment<crate::sqlite::Sqlite>,
    QS: QueryFragment<crate::sqlite::Sqlite>,
{
    fn walk_ast<'b, 'c>(&'b self, pass: AstPass<'_, 'c, crate::sqlite::Sqlite>) -> QueryResult<()>
    where
        'b: 'c,
    {
        // https://www.sqlite.org/lang_UPSERT.html (Parsing Ambiguity)
        self.0.build_query(pass, |where_clause, mut pass| {
            match where_clause {
                BoxedWhereClause::None => pass.push_sql(" WHERE 1=1 "),
                w => w.walk_ast(pass.reborrow())?,
            }
            Ok(())
        })
    }
}

impl<Inner, Tab> IntoConflictValueClause for ValuesClause<Inner, Tab> {
    type ValueClause = Self;

    fn into_value_clause(self) -> Self::ValueClause {
        self
    }
}

impl<V, Tab, QId, const STATIC_QUERY_ID: bool> IntoConflictValueClause
    for BatchInsert<V, Tab, QId, STATIC_QUERY_ID>
{
    type ValueClause = Self;

    fn into_value_clause(self) -> Self::ValueClause {
        self
    }
}

impl<F, S, D, W, O, LOf, G, H, LC, Columns> IntoConflictValueClause
    for InsertFromSelect<SelectStatement<F, S, D, W, O, LOf, G, H, LC>, Columns>
{
    type ValueClause = InsertFromSelect<
        OnConflictSelectWrapper<SelectStatement<F, S, D, W, O, LOf, G, H, LC>>,
        Columns,
    >;

    fn into_value_clause(self) -> Self::ValueClause {
        let InsertFromSelect { columns, query } = self;
        InsertFromSelect {
            query: OnConflictSelectWrapper(query),
            columns,
        }
    }
}

impl<'a, ST, QS, DB, GB, Columns> IntoConflictValueClause
    for InsertFromSelect<BoxedSelectStatement<'a, ST, QS, DB, GB>, Columns>
{
    type ValueClause = InsertFromSelect<
        OnConflictSelectWrapper<BoxedSelectStatement<'a, ST, QS, DB, GB>>,
        Columns,
    >;

    fn into_value_clause(self) -> Self::ValueClause {
        let InsertFromSelect { columns, query } = self;
        InsertFromSelect {
            query: OnConflictSelectWrapper(query),
            columns,
        }
    }
}
