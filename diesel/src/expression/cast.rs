//! SQL `CAST(expr AS sql_type)` expression support

use crate::expression::{AppearsOnTable, Expression, SelectableExpression, ValidGrouping};
use crate::query_source::aliasing::{AliasSource, FieldAliasMapper};
use crate::result::QueryResult;
use crate::{query_builder, query_source, sql_types};

use std::marker::PhantomData;

pub(crate) mod private {
    use super::*;

    #[derive(Debug, Clone, Copy, diesel::query_builder::QueryId, sql_types::DieselNumericOps)]
    pub struct Cast<E, ST> {
        pub(super) expr: E,
        pub(super) sql_type: PhantomData<ST>,
    }
}
pub(crate) use private::Cast;

impl<E, ST> Cast<E, ST> {
    pub(crate) fn new(expr: E) -> Self {
        Self {
            expr,
            sql_type: PhantomData,
        }
    }
}

impl<E, ST, GroupByClause> ValidGrouping<GroupByClause> for Cast<E, ST>
where
    E: ValidGrouping<GroupByClause>,
{
    type IsAggregate = E::IsAggregate;
}

impl<E, ST, QS> SelectableExpression<QS> for Cast<E, ST>
where
    Cast<E, ST>: AppearsOnTable<QS>,
    E: SelectableExpression<QS>,
{
}

impl<E, ST, QS> AppearsOnTable<QS> for Cast<E, ST>
where
    Cast<E, ST>: Expression,
    E: AppearsOnTable<QS>,
{
}

impl<E, ST> Expression for Cast<E, ST>
where
    E: Expression,
    ST: sql_types::SingleValue,
{
    type SqlType = ST;
}

impl<E, ST, DB> query_builder::QueryFragment<DB> for Cast<E, ST>
where
    E: query_builder::QueryFragment<DB>,
    DB: diesel::backend::Backend,
    ST: KnownCastSqlTypeName<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: query_builder::AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("CAST(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(" AS ");
        out.push_sql(ST::SQL_TYPE_NAME);
        out.push_sql(")");
        Ok(())
    }
}

/// We know what to write as `sql_type` in the `CAST(expr AS sql_type)` SQL for
/// `Self`
///
/// That is what is returned by `Self::sql_type_name()`
#[diagnostic::on_unimplemented(
    note = "In order to use `CAST`, it is necessary that Diesel knows how to express the name \
		of this type in the given backend.",
    note = "If you run into this error message and believe that this cast should be supported \
            open a PR that adds that trait implementation here: https://github.com/diesel-rs/diesel/blob/2fafe60a8f4ca3407dca5fe010a6092fa8a1858a/diesel/src/expression/cast.rs#L113."
)]
pub trait KnownCastSqlTypeName<DB> {
    /// What to write as `sql_type` in the `CAST(expr AS sql_type)` SQL for
    /// `Self`
    const SQL_TYPE_NAME: &'static str;
}

impl<ST, DB> KnownCastSqlTypeName<DB> for sql_types::Nullable<ST>
where
    ST: KnownCastSqlTypeName<DB>,
{
    const SQL_TYPE_NAME: &'static str = <ST as KnownCastSqlTypeName<DB>>::SQL_TYPE_NAME;
}

macro_rules! type_name {
    ($($backend: ty: $backend_feature: literal { $($type: ident => $val: literal,)+ })*) => {
        $(
            $(
				#[cfg(feature = $backend_feature)]
                impl KnownCastSqlTypeName<$backend> for sql_types::$type {
                    const SQL_TYPE_NAME: &'static str = $val;
                }
            )*
        )*
    };
}
type_name! {
    diesel::pg::Pg: "postgres_backend" {
        Bool => "bool",
        Int2 => "int2",
        Int4 => "int4",
        Int8 => "int8",
        Float => "float4",
        Double => "float8",
        Numeric => "numeric",
        Text => "text",
        Date => "date",
        Interval => "interval",
        Time => "time",
        Timestamp => "timestamp",
        Uuid => "uuid",
        Json => "json",
        Jsonb => "jsonb",
    }
    diesel::mysql::Mysql: "mysql_backend" {
        Int4 => "integer",
        Int8 => "integer",
        Text => "char",

    }
    diesel::sqlite::Sqlite: "sqlite" {
        Int4 => "integer",
        Int8 => "bigint",
        Text => "text",
    }
}

impl<S, E, ST> FieldAliasMapper<S> for Cast<E, ST>
where
    S: AliasSource,
    E: FieldAliasMapper<S>,
{
    type Out = Cast<<E as FieldAliasMapper<S>>::Out, ST>;

    fn map(self, alias: &query_source::Alias<S>) -> Self::Out {
        Cast {
            expr: self.expr.map(alias),
            sql_type: self.sql_type,
        }
    }
}

/// Marker trait: this SQL type (`Self`) can be casted to the target SQL type
/// (`ST`) using `CAST(expr AS target_sql_type)`
pub trait CastsTo<ST> {}

impl<ST1, ST2> CastsTo<sql_types::Nullable<ST2>> for sql_types::Nullable<ST1> where ST1: CastsTo<ST2>
{}

impl CastsTo<sql_types::Int8> for sql_types::Int4 {}
impl CastsTo<sql_types::Int8> for sql_types::Text {}
impl CastsTo<sql_types::Int4> for sql_types::Int8 {}
impl CastsTo<sql_types::Int4> for sql_types::Text {}
impl CastsTo<sql_types::Uuid> for sql_types::Text {}
impl CastsTo<sql_types::Text> for sql_types::Int4 {}
impl CastsTo<sql_types::Text> for sql_types::Int8 {}
impl CastsTo<sql_types::Text> for sql_types::Uuid {}
impl CastsTo<sql_types::Text> for sql_types::Jsonb {}
impl CastsTo<sql_types::Text> for sql_types::Json {}
impl CastsTo<sql_types::Jsonb> for sql_types::Text {}
impl CastsTo<sql_types::Json> for sql_types::Text {}
