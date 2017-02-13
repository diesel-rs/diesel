table! {
    __diesel_schema_migrations (version) {
        version -> VarChar,
        run_on -> Timestamp,
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NewMigration<'a>(pub &'a str);

use insertable::{Insertable, InsertValues, ColumnInsertValue};
use backend::Backend;
use query_builder::insert_statement::{IntoInsertStatement, InsertStatement};
use expression::helper_types::AsNullableExpr;

type Table = self::__diesel_schema_migrations::table;
type Version = self::__diesel_schema_migrations::version;

impl<'insert, 'a, DB> Insertable<Table, DB> for &'insert NewMigration<'a> where
    DB: Backend,
    (ColumnInsertValue<
        Version,
        AsNullableExpr<&'insert &'a str,Version>,
      >, ): InsertValues<DB>,
{
    type Values = (
        ColumnInsertValue<
            Version,
            AsNullableExpr<&'insert &'a str, Version>,
        >, );

    fn values(self) -> Self::Values {
        use expression::{AsExpression, Expression};
        use insertable::ColumnInsertValue;
        use types::IntoNullable;

        type SqlType<T> = <T as Expression>::SqlType;
        type Nullable<T> = <T as IntoNullable>::Nullable;

        let ref version = self.0;
        (ColumnInsertValue::Expression(self::__diesel_schema_migrations::version,
            AsExpression::<Nullable<SqlType<Version>>>::as_expression(version)),)
    }
}

impl<'i, 'a:'i, Op, Ret> IntoInsertStatement<Table, Op, Ret> for &'i NewMigration<'a> {
    type InsertStatement = InsertStatement<Table, Self, Op, Ret>;

    fn into_insert_statement(self, t: Table, op: Op, ret: Ret) -> Self::InsertStatement {
        InsertStatement::new(t, self, op, ret)
    }
}
