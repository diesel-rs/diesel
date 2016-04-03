use diesel::*;
#[cfg(feature = "postgres")]
use diesel::pg::Pg;
#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
use diesel::sqlite::Sqlite;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;
use diesel::types::{HasSqlType, FromSqlRow};

#[derive(Debug, Clone)]
pub struct ColumnInformation {
    pub column_name: String,
    pub type_name: String,
    pub nullable: bool,
}

#[cfg(feature = "postgres")]
impl<ST> Queryable<ST, Pg> for ColumnInformation where
    Pg: HasSqlType<ST>,
    (String, String, bool): FromSqlRow<ST, Pg>,
{
    type Row = (String, String, bool);

    fn build(row: Self::Row) -> Self {
        ColumnInformation {
            column_name: row.0,
            type_name: row.1,
            nullable: !row.2,
        }
    }
}

#[cfg(feature = "sqlite")]
impl<ST> Queryable<ST, Sqlite> for ColumnInformation where
    Sqlite: HasSqlType<ST>,
    (i32, String, String, bool, Option<String>, bool): FromSqlRow<ST, Sqlite>,
{
    type Row = (i32, String, String, bool, Option<String>, bool);

    fn build(row: Self::Row) -> Self {
        ColumnInformation {
            column_name: row.1,
            type_name: row.2,
            nullable: !row.3,
        }
    }
}

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub enum InferConnection {
    Sqlite(SqliteConnection),
}

#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub enum InferConnection {
    Pg(PgConnection),
}

#[cfg(all(feature = "sqlite", feature = "postgres"))]
pub enum InferConnection {
    Sqlite(SqliteConnection),
    Pg(PgConnection),
}
