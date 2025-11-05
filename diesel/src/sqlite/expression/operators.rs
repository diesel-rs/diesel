use crate::sql_types::Bool;
use crate::sqlite::Sqlite;

__diesel_infix_operator!(Is, " IS ", ConstantNullability Bool, backend: Sqlite);
__diesel_infix_operator!(IsNot, " IS NOT ", ConstantNullability Bool, backend: Sqlite);
// RetrieveAsObjectJson and RetrieveAsTextJson have been moved to crate::expression::operators
// to avoid conflicts when both postgres_backend and sqlite features are enabled
// SQLite's -> operator always returns TEXT JSON representation, not JSONB
// See: https://www.sqlite.org/json1.html#jptr
