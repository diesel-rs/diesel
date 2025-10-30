use crate::sql_types::{Bool, Json, Text};
use crate::sqlite::Sqlite;

__diesel_infix_operator!(Is, " IS ", ConstantNullability Bool, backend: Sqlite);
__diesel_infix_operator!(IsNot, " IS NOT ", ConstantNullability Bool, backend: Sqlite);
// SQLite's -> operator always returns TEXT JSON representation, not JSONB
// See: https://www.sqlite.org/json1.html#jptr
infix_operator!(RetrieveAsObjectJson, " -> ", Json, backend: Sqlite);
infix_operator!(RetrieveAsTextJson, " ->> ", Text, backend: Sqlite);
