use diesel::pg::PgValue;
use diesel::serialize::{self, IsNull, Output, ToSql};
use std::io::Write;

pub mod exports {
    pub use super::LanguageType as Language;
}

#[derive(SqlType)]
#[postgres(type_name = "Language")]
pub struct LanguageType;

#[derive(Debug, FromSqlRow, AsExpression)]
#[sql_type = "LanguageType"]
pub enum Language {
    En,
    Ru,
    De,
}

impl ToSql<LanguageType, Pg> for Language {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            Language::En => out.write_all(b"en")?,
            Language::Ru => out.write_all(b"ru")?,
            Language::De => out.write_all(b"de")?,
        }
        Ok(IsNull::No)
    }
}

use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;

impl FromSql<LanguageType, Pg> for Language {
    fn from_sql(bytes: Option<PgValue>) -> deserialize::Result<Self> {
        match not_none!(bytes).as_bytes() {
            b"en" => Ok(Language::En),
            b"ru" => Ok(Language::Ru),
            b"de" => Ok(Language::De),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
