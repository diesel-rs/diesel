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

use std::io::Write;

use diesel::backend::Backend;
use diesel::serialize::{self, IsNull, Output, ToSql};

impl<Db: Backend> ToSql<LanguageType, Db> for Language {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Db>) -> serialize::Result {
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
    fn from_sql(bytes: Option<&<Pg as Backend>::RawValue>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"en" => Ok(Language::En),
            b"ru" => Ok(Language::Ru),
            b"de" => Ok(Language::De),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
