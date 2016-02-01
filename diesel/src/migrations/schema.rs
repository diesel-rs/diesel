table! {
    __diesel_schema_migrations (version) {
        version -> VarChar,
        run_on -> Timestamp,
    }
}

pub struct NewMigration<'a>(pub &'a str);

use backend::Backend;
use expression::AsExpression;
use expression::helper_types::AsExpr;
use persistable::{Insertable, ColumnInsertValue, InsertValues};

impl<'update: 'a, 'a, DB> Insertable<__diesel_schema_migrations::table, DB>
    for &'update NewMigration<'a> where
        DB: Backend,
        (ColumnInsertValue<
            __diesel_schema_migrations::version,
            AsExpr<&'a str, __diesel_schema_migrations::version>,
        >,): InsertValues<DB>,
{
    type Values = (ColumnInsertValue<
        __diesel_schema_migrations::version,
        AsExpr<&'a str, __diesel_schema_migrations::version>,
    >,);

    fn values(self) -> Self::Values {
        (ColumnInsertValue::Expression(
            __diesel_schema_migrations::version,
            AsExpression::<::types::VarChar>::as_expression(self.0),
        ),)
    }
}
