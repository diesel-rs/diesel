use crate::sql_types::Bool;
use crate::sqlite::Sqlite;

__diesel_infix_operator!(Is, " IS ", ConstantNullability Bool, backend: Sqlite);
__diesel_infix_operator!(IsNot, " IS NOT ", ConstantNullability Bool, backend: Sqlite);
