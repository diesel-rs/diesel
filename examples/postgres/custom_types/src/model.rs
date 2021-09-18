use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, IsNull, Output, ToSql};
use std::io::Write;

#[derive(Debug, AsExpression, FromSqlRow)]
#[sql_type = "crate::schema::sql_types::Language"]
pub enum Language {
    En,
    Ru,
    De,
}

impl ToSql<crate::schema::sql_types::Language, Pg> for Language {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            Language::En => out.write_all(b"en")?,
            Language::Ru => out.write_all(b"ru")?,
            Language::De => out.write_all(b"de")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::Language, Pg> for Language {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"en" => Ok(Language::En),
            b"ru" => Ok(Language::Ru),
            b"de" => Ok(Language::De),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
