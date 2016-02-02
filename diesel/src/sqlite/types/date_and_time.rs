use sqlite::{Sqlite, SqliteType};
use types;

impl types::HasSqlType<types::Date> for Sqlite {
    fn metadata() -> SqliteType {
        SqliteType::Text
    }
}

impl types::HasSqlType<types::Time> for Sqlite {
    fn metadata() -> SqliteType {
        SqliteType::Text
    }
}

impl types::HasSqlType<types::Timestamp> for Sqlite {
    fn metadata() -> SqliteType {
        SqliteType::Text
    }
}
