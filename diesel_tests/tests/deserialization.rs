use crate::schema::*;
use diesel::deserialize::FromSqlRow;
use diesel::prelude::*;
use std::borrow::Cow;

#[derive(Queryable, PartialEq, Debug, Selectable)]
#[table_name = "users"]
struct CowUser<'a> {
    id: i32,
    name: Cow<'a, str>,
}

#[test]
fn generated_queryable_allows_lifetimes() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let expected_user = CowUser {
        id: 1,
        name: Cow::Owned("Sean".to_string()),
    };
    assert_eq!(
        Ok(expected_user),
        users.select((id, name)).first(connection)
    );
    assert_eq!(
        users.select((id, name)).first::<CowUser>(connection),
        users.select(CowUser::as_select()).first(connection)
    );
}

#[test]
fn fun_with_row_iters() {
    use crate::schema::users::dsl::*;
    use diesel::deserialize::FromSql;
    use diesel::row::{Field, Row};
    use diesel::sql_types;

    let conn = &mut connection_with_sean_and_tess_in_users_table();

    let query = users.select((id, name));

    let expected = vec![(1, String::from("Sean")), (2, String::from("Tess"))];

    let row_iter = conn.load(&query).unwrap();
    for (row, expected) in row_iter.zip(&expected) {
        let row = row.unwrap();

        let deserialized = <(i32, String) as FromSqlRow<
            (sql_types::Integer, sql_types::Text),
            _,
        >>::build_from_row(&row)
        .unwrap();

        assert_eq!(&deserialized, expected);
    }

    {
        let collected_rows = conn.load(&query).unwrap().collect::<Vec<_>>();

        for (row, expected) in collected_rows.iter().zip(&expected) {
            let deserialized = row
                .as_ref()
                .map(|row| {
                    <(i32, String) as FromSqlRow<
                            (sql_types::Integer, sql_types::Text),
                        _,
                        >>::build_from_row(row).unwrap()
                })
                .unwrap();

            assert_eq!(&deserialized, expected);
        }
    }

    let mut row_iter = conn.load(&query).unwrap();

    dbg!();
    let first_row = row_iter.next().unwrap().unwrap();
    let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
    let first_values = (first_fields.0.value(), first_fields.1.value());

    dbg!();
    let second_row = row_iter.next().unwrap().unwrap();
    let second_fields = (second_row.get(0).unwrap(), second_row.get(1).unwrap());
    let second_values = (second_fields.0.value(), second_fields.1.value());

    assert!(row_iter.next().is_none());
    dbg!(
        <i32 as FromSql<sql_types::Integer, TestBackend>>::from_nullable_sql(first_values.0)
            .unwrap()
    ); //, expected[0].0);
    dbg!(
        <String as FromSql<sql_types::Text, TestBackend>>::from_nullable_sql(first_values.1)
            .unwrap()
    ); //, expected[0].1);

    dbg!(
        <i32 as FromSql<sql_types::Integer, TestBackend>>::from_nullable_sql(second_values.0)
            .unwrap()
    ); //, expected[1].0);
    dbg!(
        <String as FromSql<sql_types::Text, TestBackend>>::from_nullable_sql(second_values.1)
            .unwrap()
    ); //, expected[1].1);

    let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
    let first_values = (first_fields.0.value(), first_fields.1.value());

    dbg!(
        <i32 as FromSql<sql_types::Integer, TestBackend>>::from_nullable_sql(first_values.0)
            .unwrap()
    ); //, expected[0].0);
    dbg!(
        <String as FromSql<sql_types::Text, TestBackend>>::from_nullable_sql(first_values.1)
            .unwrap()
    ); //, expected[0].1);

    panic!()
}
