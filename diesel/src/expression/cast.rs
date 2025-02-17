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
        out.push_sql(ST::sql_type_name());
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
    note = "This can be PRed into Diesel if the type is a standard SQL type."
)]
pub trait KnownCastSqlTypeName<DB> {
    /// What to write as `sql_type` in the `CAST(expr AS sql_type)` SQL for
    /// `Self`
    fn sql_type_name() -> &'static str;
}

impl<ST, DB> KnownCastSqlTypeName<DB> for sql_types::Nullable<ST>
where
    ST: KnownCastSqlTypeName<DB>,
{
    fn sql_type_name() -> &'static str {
        <ST as KnownCastSqlTypeName<DB>>::sql_type_name()
    }
}

macro_rules! type_name {
    ($($backend: ty: $backend_feature: literal { $($type: ident => $val: literal,)+ })*) => {
        $(
            $(
				#[cfg(feature = $backend_feature)]
                impl KnownCastSqlTypeName<$backend> for sql_types::$type {
                    fn sql_type_name() -> &'static str {
                        $val
                    }
                }
            )*
        )*
    };
}
type_name! {
    diesel::pg::Pg: "postgres_backend" {
        Int4 => "int4",
        Int8 => "int8",
        Text => "text",
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
impl CastsTo<sql_types::Int4> for sql_types::Int8 {}
impl CastsTo<sql_types::Text> for sql_types::Int4 {}
impl CastsTo<sql_types::Text> for sql_types::Int8 {}
