use diesel::deserialize::*;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;
use diesel_dynamic_schema::dynamic_value::*;
use diesel_dynamic_schema::DynamicSelectClause;

#[derive(PartialEq, Debug)]
enum MyDynamicValue {
    String(String),
    Integer(i32),
}

#[cfg(feature = "postgres")]
impl FromSql<Any, diesel::pg::Pg> for MyDynamicValue {
    fn from_sql(value: diesel::pg::PgValue) -> Result<Self> {
        use diesel::pg::Pg;
        use std::num::NonZeroU32;

        const VARCHAR_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1043) };
        const TEXT_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(25) };
        const INTEGER_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(23) };

        match value.get_oid() {
            VARCHAR_OID | TEXT_OID => {
                <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            INTEGER_OID => <i32 as FromSql<diesel::sql_types::Integer, Pg>>::from_sql(value)
                .map(MyDynamicValue::Integer),
            e => Err(format!("Unknown type: {}", e).into()),
        }
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<Any, diesel::sqlite::Sqlite> for MyDynamicValue {
    fn from_sql(value: diesel::sqlite::SqliteValue) -> Result<Self> {
        use diesel::sqlite::{Sqlite, SqliteType};
        match value.value_type() {
            Some(SqliteType::Text) => {
                <String as FromSql<diesel::sql_types::Text, Sqlite>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            Some(SqliteType::Long) => {
                <i32 as FromSql<diesel::sql_types::Integer, Sqlite>>::from_sql(value)
                    .map(MyDynamicValue::Integer)
            }
            _ => Err("Unknown data type".into()),
        }
    }
}

#[cfg(feature = "mysql")]
impl FromSql<Any, diesel::mysql::Mysql> for MyDynamicValue {
    fn from_sql(value: diesel::mysql::MysqlValue) -> Result<Self> {
        use diesel::mysql::{Mysql, MysqlType};
        match value.value_type() {
            MysqlType::String => {
                <String as FromSql<diesel::sql_types::Text, Mysql>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            MysqlType::Long => <i32 as FromSql<diesel::sql_types::Integer, Mysql>>::from_sql(value)
                .map(MyDynamicValue::Integer),
            e => Err(format!("Unknown data type: {:?}", e).into()),
        }
    }
}

#[test]
fn dynamic_query() {
    let connection = &mut super::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (name, hair_color) VALUES ('Sean', 'black'), ('Tess', 'black')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let id = users.column::<Untyped, _>("id");
    let name = users.column::<Untyped, _>("name");
    let hair_color = users.column::<Untyped, _>("hair_color");

    let mut select = DynamicSelectClause::new();

    select.add_field(id);
    select.add_field(name);
    select.add_field(hair_color);

    let actual_data: Vec<DynamicRow<NamedField<MyDynamicValue>>> =
        users.select(select).load(connection).unwrap();

    assert_eq!(
        actual_data[0]["name"],
        MyDynamicValue::String("Sean".into())
    );
    assert_eq!(
        actual_data[0][1],
        NamedField {
            name: "name".into(),
            value: MyDynamicValue::String("Sean".into())
        }
    );
    assert_eq!(
        actual_data[1]["name"],
        MyDynamicValue::String("Tess".into())
    );
    assert_eq!(
        actual_data[1][1],
        NamedField {
            name: "name".into(),
            value: MyDynamicValue::String("Tess".into())
        }
    );
    assert_eq!(
        actual_data[0]["hair_color"],
        MyDynamicValue::String("black".into())
    );
    assert_eq!(
        actual_data[0][2],
        NamedField {
            name: "hair_color".into(),
            value: MyDynamicValue::String("black".into())
        }
    );
    assert_eq!(
        actual_data[1]["hair_color"],
        MyDynamicValue::String("black".into())
    );
    assert_eq!(
        actual_data[1][2],
        NamedField {
            name: "hair_color".into(),
            value: MyDynamicValue::String("black".into())
        }
    );

    let mut select = DynamicSelectClause::new();

    select.add_field(id);
    select.add_field(name);
    select.add_field(hair_color);

    let actual_data: Vec<DynamicRow<MyDynamicValue>> =
        users.select(select).load(connection).unwrap();

    assert_eq!(actual_data[0][1], MyDynamicValue::String("Sean".into()));
    assert_eq!(actual_data[1][1], MyDynamicValue::String("Tess".into()));
    assert_eq!(actual_data[0][2], MyDynamicValue::String("black".into()));
    assert_eq!(actual_data[1][2], MyDynamicValue::String("black".into()));
}

#[test]
fn mixed_value_query() {
    use diesel::dsl::sql;

    let connection = &mut crate::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (id, name, hair_color) VALUES (42, 'Sean', 'black'), (43, 'Tess', 'black')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let id = users.column::<Integer, _>("id");

    let (id, row) = users
        .select((id, sql::<Untyped>("name, hair_color")))
        .first::<(i32, DynamicRow<NamedField<MyDynamicValue>>)>(connection)
        .unwrap();

    assert_eq!(id, 42);
    assert_eq!(row["name"], MyDynamicValue::String("Sean".into()));
    assert_eq!(row["hair_color"], MyDynamicValue::String("black".into()));
}

#[test]
fn nullable_dynamic_value() {
    use diesel::dsl::sql;

    let connection = &mut crate::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (name, hair_color) VALUES ('Sean', 'dark'), ('Tess', NULL)")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");

    let result = users
        .select(sql::<Untyped>("hair_color"))
        .load::<DynamicRow<Option<MyDynamicValue>>>(connection)
        .unwrap();

    assert_eq!(result[0][0], Some(MyDynamicValue::String("dark".into())));
    assert_eq!(result[1][0], None);

    let result = users
        .select(sql::<Untyped>("hair_color"))
        .load::<DynamicRow<NamedField<Option<MyDynamicValue>>>>(connection)
        .unwrap();

    assert_eq!(
        result[0]["hair_color"],
        Some(MyDynamicValue::String("dark".into()))
    );
    assert_eq!(result[1]["hair_color"], None);
}
