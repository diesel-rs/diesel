use std::fmt::{Debug, Display};
use std::io::Write;
use std::str;
use std::str::FromStr;
use crate::{deserialize, serialize, sql_types};
use crate::deserialize::FromSql;
use crate::pg::Pg;
use crate::serialize::{Output, ToSql};

#[cfg(all(feature = "postgres_backend", feature = "serde_json"))]
impl FromSql<sql_types::Jsonpath, Pg> for jsonpath_rust::JsonPath {
    fn from_sql(value: Pg::RawValue<'_>) -> deserialize::Result<Self> {
        let str = str::from_utf8(value.as_bytes()).map_err(|_| "Invalid path".into())?;
        jsonpath_rust::JsonPath::from_str(str).map_err(|_| "Invalid path".into())
    }
}

impl ToSql<sql_types::Jsonpath, Pg> for jsonpath_rust::JsonPath {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        
    }
}



#[cfg(test)]
mod tests {
    use jsonpath_rust::JsonPath;

    use crate::pg::Pg;
    use crate::query_builder::bind_collector::ByteWrapper;
    use crate::serialize::{Output, ToSql};
    use crate::sql_types;

    #[test]
    fn json_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_json_path = jsonpath_rust::path!("$.abc");
        ToSql::<sql_types::Jsonpath, Pg>::to_sql(&test_json_path, &mut bytes).unwrap();
        assert_eq!(buffer, b"$.abc");
    }
}
