use crate::sql_types::Bool;
use crate::sql_types::Json;
use crate::sqlite::Sqlite;

__diesel_infix_operator!(Is, " IS ", ConstantNullability Bool, backend: Sqlite);
__diesel_infix_operator!(IsNot, " IS NOT ", ConstantNullability Bool, backend: Sqlite);
infix_operator!(RetrieveAsObjectSqlite, " -> ", Json, backend: Sqlite);
