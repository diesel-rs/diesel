use diesel::*;
use diesel::pg::Pg;
use diesel::types::{FromSqlRow, HasSqlType};

table! {
    pg_attribute (attrelid) {
        attrelid -> Oid,
        attname -> VarChar,
        atttypid -> Oid,
        attnotnull -> Bool,
        attnum -> SmallInt,
        attisdropped -> Bool,
    }
}

table! {
    pg_type (oid) {
        oid -> Oid,
        typname -> VarChar,
    }
}

joinable!(pg_attribute -> pg_type (atttypid));
select_column_workaround!(pg_attribute -> pg_type (attrelid, attname, atttypid, attnotnull, attnum, attisdropped));
select_column_workaround!(pg_type -> pg_attribute (oid, typname));

table! {
    pg_class (oid) {
        oid -> Oid,
        relname -> VarChar,
    }
}

#[derive(Debug, Clone)]
pub struct PgAttr {
    pub column_name: String,
    pub type_name: String,
    pub nullable: bool,
}

impl<ST> Queryable<ST, Pg> for PgAttr
    where Pg: HasSqlType<ST>,
          (String, String, bool): FromSqlRow<ST, Pg>,
{
    type Row = (String, String, bool);

    fn build(row: Self::Row) -> Self {
        PgAttr {
            column_name: row.0,
            type_name: row.1,
            nullable: !row.2,
        }
    }
}
