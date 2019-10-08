/// The JSON SQL type.  This type can only be used with `feature =
/// "serde_json"`
///
/// Normally you should prefer [`Jsonb`](struct.Jsonb.html) instead, for the reasons
/// discussed there.
///
/// ### [`ToSql`] impls
///
/// - [`serde_json::Value`]
///
/// ### [`FromSql`] impls
///
/// - [`serde_json::Value`]
///
/// [`ToSql`]: ../../../serialize/trait.ToSql.html
/// [`FromSql`]: ../../../deserialize/trait.FromSql.html
/// [`serde_json::Value`]: ../../../../serde_json/value/enum.Value.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "114", array_oid = "199")]
#[sqlite_type = "Text"]
#[mysql_type = "String"]
#[cfg(feature = "serde_json")]
pub struct Json;

#[allow(dead_code)]
mod foreign_derives {
    use super::Json;

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Json"]
    struct SerdeJsonValueProxy(serde_json::Value);
}
