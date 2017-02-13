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

impl<'insert, 'a, DB> Insertable<self::__diesel_schema_migrations::table, DB>
    for &'insert NewMigration<'a>
    where DB: Backend,
               (ColumnInsertValue<self::__diesel_schema_migrations::version,
                   AsNullableExpr<&'insert &'a str,
                       self::__diesel_schema_migrations::version>>, ): InsertValues<DB>,
{
    type Values = (ColumnInsertValue<self::__diesel_schema_migrations::version,
                   AsNullableExpr<&'insert &'a str,
                                  self::__diesel_schema_migrations::version>>, );

    fn values(self) -> Self::Values {
        use expression::{AsExpression, Expression};
        use insertable::ColumnInsertValue;
        use types::IntoNullable;
        let ref version = self.0;
        (ColumnInsertValue::Expression(self::__diesel_schema_migrations::version,
            AsExpression::<<<self::__diesel_schema_migrations::version
                as Expression>::SqlType as IntoNullable>::Nullable>
            ::as_expression(version)),)
    }
}

impl<'insert, 'a:'insert, Op, Ret> IntoInsertStatement<
    self::__diesel_schema_migrations::table,
    Op,
    Ret
> for &'insert NewMigration<'a> {
    type InsertStatement = InsertStatement<
        self::__diesel_schema_migrations::table,
        Self,
        Op,
        Ret
    >;

    fn into_insert_statement(
        self,
        target: self::__diesel_schema_migrations::table,
        operator: Op,
        returning: Ret
    ) -> Self::InsertStatement {
        InsertStatement::new(target, self, operator, returning)
    }
}
