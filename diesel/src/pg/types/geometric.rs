//! Support for Geometric types under PostgreSQL.

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::prelude::*;

use deserialize::{self, FromSql};
use pg::Pg;
use serialize::{self, IsNull, Output, ToSql};
use sql_types::Point;

/// Point is represented in Postgres as a tuple of 64 bit floating point values (x, y).  This
/// struct is a dumb wrapper type, meant only to indicate the tuple's meaning.
#[derive(Debug, Clone, PartialEq, Copy, FromSqlRow, AsExpression)]
#[sql_type = "Point"]
pub struct PgPoint(pub f64, pub f64);

impl FromSql<Point, Pg> for PgPoint {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        let x = bytes.read_f64::<NetworkEndian>()?;
        let y = bytes.read_f64::<NetworkEndian>()?;
        Ok(PgPoint(x, y))
    }
}

impl ToSql<Point, Pg> for PgPoint {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_f64::<NetworkEndian>(self.0)?;
        out.write_f64::<NetworkEndian>(self.1)?;
        Ok(IsNull::No)
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenv;

    use self::dotenv::dotenv;

    use deserialize::FromSql;
    use dsl::sql;
    use pg::Pg;
    use prelude::*;
    use select;
    use serialize::{Output, ToSql};
    use sql_types::Point;
    use super::PgPoint;

    fn connection() -> PgConnection {
        dotenv().ok();

        let connection_url = ::std::env::var("PG_DATABASE_URL")
            .or_else(|_| ::std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        PgConnection::establish(&connection_url).unwrap()
    }

    diesel_infix_operator!(SameAs, " ~= ");

    use expression::AsExpression;

    // Normally you would put this on a trait instead
    fn same_as<T, U>(left: T, right: U) -> SameAs<T, U::Expression>
    where
        T: Expression,
        U: AsExpression<T::SqlType>,
    {
        SameAs::new(left, right.as_expression())
    }

    #[test]
    fn point_roundtrip() {
        let mut bytes = Output::test();
        let input_point = PgPoint(4.5, 3439.1);
        ToSql::<Point, Pg>::to_sql(&input_point, &mut bytes).unwrap();
        let output_point: PgPoint = FromSql::from_sql(Some(bytes.as_ref())).unwrap();
        assert_eq!(input_point, output_point);
    }

    #[test]
    fn no_point_from_sql() {
        let uuid: Result<PgPoint, _> = FromSql::<Point, Pg>::from_sql(None);
        assert_eq!(
            uuid.unwrap_err().description(),
            "Unexpected null for non-null column"
        );
    }

    #[test]
    fn point_encodes_correctly() {
        let connection = connection();
        let point = PgPoint(3.0, 4.0);
        let query = select(same_as(sql::<Point>("point '(3, 4)'"), point));
        println!("{:?}", ::debug_query(&query));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    mod schema {
        table! {
            items {
                id -> Integer,
                name -> VarChar,
                location -> Point,
            }
        }
    }

    #[test]
    fn point_is_insertable() {
        // Compile check that PgPoint can be used in insertable context,
        use self::schema::items;
        #[derive(Debug, Clone, Copy, Insertable)]
        #[table_name = "items"]
        struct NewItem {
            name: &'static str,
            location: PgPoint,
        }
        use self::schema::items::dsl::*;
        let _query_location = ::insert_into(items)
            .values(&NewItem {
                name: "Shiny Thing",
                location: PgPoint(3.1, 9.4),
            })
            .returning(location);
    }

    #[test]
    fn point_is_queryable() {
        let connection = connection();
        // Compile check that PgPoint can be used in queryable context,
        #[derive(Debug, Clone, Queryable)]
        struct Item {
            id: i32,
            name: String,
            location: PgPoint,
        }
        use self::schema::items::dsl::*;
        let _query_row = items
            .filter(id.eq(1))
            .filter(same_as(location, PgPoint(3.1, 9.4)))
            .get_result::<Item>(&connection);
    }
}
