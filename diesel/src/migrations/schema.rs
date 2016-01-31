table! {
    __diesel_schema_migrations (version) {
        version -> VarChar,
        run_on -> Timestamp,
    }
}

pub struct NewMigration<'a>(pub &'a str);

use expression::AsExpression;
use expression::grouped::Grouped;
use expression::helper_types::AsExpr;
use {Insertable, types};

impl<'update: 'a, 'a, DB> Insertable<__diesel_schema_migrations::table, DB>
for &'update NewMigration<'a> {
    type Columns = __diesel_schema_migrations::version;
    type Values = Grouped<AsExpr<&'a str, Self::Columns>>;

    fn columns() -> Self::Columns {
        __diesel_schema_migrations::version
    }

    fn values(self) -> Self::Values {
        Grouped(AsExpression::<types::VarChar>::as_expression(self.0))
    }
}
