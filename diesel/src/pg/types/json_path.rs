use crate::deserialize::FromSql;
use crate::pg::{backend::Pg, PgValue};
use crate::serialize::{IsNull, Output, ToSql};
use crate::{deserialize, serialize, sql_types};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::str;
use std::str::FromStr;

impl FromSql<sql_types::Jsonpath, Pg> for jsonpath_rust::JsonPath {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        println!("{:?}", value.as_bytes());
        let str = str::from_utf8(value.as_bytes())
            .map_err(|_| <&str as Into<Box<dyn Error + Send + Sync>>>::into("Invalid path"))?;
        println!("{}", str);
        jsonpath_rust::JsonPath::from_str(str).map_err(|_| "Invalid path".into())
    }
}

impl ToSql<sql_types::Jsonpath, Pg> for jsonpath_rust::JsonPath {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        println!("{}", self);
        out.write_all(self.to_string().as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pg::PgValue;
    use crate::query_builder::bind_collector::ByteWrapper;
    use jsonpath_rust::{path, JsonPath};

    #[test]
    fn json_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_json_path = JsonPath::from_str("$.a[?(@ >= 2 && @ <= 4)]").unwrap();
        ToSql::<sql_types::Jsonpath, Pg>::to_sql(&test_json_path, &mut bytes).unwrap();
        println!("{}",String::from_utf8(buffer.clone()).unwrap());
        assert_eq!(buffer, b"$.'a'[?(@ >= 2 && @ <= 4)]");
    }

    #[test]
    fn path_from_sql() {
        let path = b"$.abc";
        let path: JsonPath =
            FromSql::<sql_types::Jsonpath, Pg>::from_sql(PgValue::for_test(path)).unwrap();
        assert_eq!(path, JsonPath::from_str("$.abc").unwrap());
    }
}
