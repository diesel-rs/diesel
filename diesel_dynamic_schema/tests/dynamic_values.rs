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

        const VARCHAR_OID: NonZeroU32 = NonZeroU32::new(1043).unwrap();
        const TEXT_OID: NonZeroU32 = NonZeroU32::new(25).unwrap();
        const INTEGER_OID: NonZeroU32 = NonZeroU32::new(23).unwrap();

        match value.get_oid() {
            VARCHAR_OID | TEXT_OID => {
                <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            INTEGER_OID => <i32 as FromSql<diesel::sql_types::Integer, Pg>>::from_sql(value)
                .map(MyDynamicValue::Integer),
            e => Err(format!("Unknown type: {e}").into()),
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
            e => Err(format!("Unknown data type: {e:?}").into()),
        }
    }
}

#[cfg(feature = "postgres")]
type TestDB = diesel::pg::Pg;
#[cfg(feature = "mysql")]
type TestDB = diesel::mysql::Mysql;
#[cfg(feature = "sqlite")]
type TestDB = diesel::sqlite::Sqlite;

#[test]
#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
fn test_ergonomics() {
    let connection = &mut super::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (name, hair_color) VALUES ('Sean', 'black'), ('Tess', 'black')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let name = users.column::<Untyped, _>("name");

    // Test DynamicSelectClause: Extend & IntoIterator
    let mut select_clause: DynamicSelectClause<TestDB, diesel_dynamic_schema::Table<&str>> =
        DynamicSelectClause::new();
    select_clause.add_field(name);

    // Extend
    let hair = users.column::<Untyped, _>("hair_color");
    let fields = vec![hair];
    select_clause.extend(fields);

    // IntoIterator
    assert_eq!(select_clause.into_iter().count(), 2);

    // Test DynamicRow ergonomics
    // Re-create query since select_clause was consumed
    let name = users.column::<Untyped, _>("name");
    let hair_color = users.column::<Untyped, _>("hair_color");
    let mut select = DynamicSelectClause::new();
    select.add_fields(vec![name, hair_color]);

    let mut actual_data: Vec<DynamicRow<NamedField<MyDynamicValue>>> =
        users.select(select).load(connection).unwrap();

    let row = &mut actual_data[0];

    // IndexMut (usize)
    if let MyDynamicValue::String(ref mut s) = row[0].value {
        *s = "UpdatedName".to_string();
    }
    assert_eq!(
        row[0].value,
        MyDynamicValue::String("UpdatedName".to_string())
    );

    // IndexMut (str)
    if let MyDynamicValue::String(ref mut s) = row["hair_color"] {
        *s = "UpdatedHair".to_string();
    }
    assert_eq!(
        row["hair_color"],
        MyDynamicValue::String("UpdatedHair".to_string())
    );

    // Deref/DerefMut
    let field = &mut row[0];
    if let MyDynamicValue::String(ref mut s) = **field {
        *s = "DerefUpdated".to_string();
    }
    // Check via deref
    assert_eq!(**field, MyDynamicValue::String("DerefUpdated".to_string()));

    // Iter/IterMut on Row
    for field in row.iter_mut() {
        // field is &mut NamedField<MyDynamicValue>
        if let MyDynamicValue::String(ref mut s) = field.value {
            *s = format!("Iter_{}", s);
        }
    }
    assert_eq!(
        row[0].value,
        MyDynamicValue::String("Iter_DerefUpdated".to_string())
    );

    // IntoIterator for &mut DynamicRow
    for field in &mut *row {
        // field is &mut NamedField<MyDynamicValue>
        if let MyDynamicValue::String(ref mut s) = field.value {
            *s = format!("IntoIter_{}", s);
        }
    }
    assert_eq!(
        row[0].value,
        MyDynamicValue::String("IntoIter_Iter_DerefUpdated".to_string())
    );

    // From<Vec> and Into<Vec>
    // Construct simple row without names
    let raw_vec = vec![MyDynamicValue::Integer(1), MyDynamicValue::Integer(2)];
    let dyn_row: DynamicRow<MyDynamicValue> = raw_vec.into();
    let back_to_vec: Vec<MyDynamicValue> = dyn_row.into();
    assert_eq!(back_to_vec.len(), 2);
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
