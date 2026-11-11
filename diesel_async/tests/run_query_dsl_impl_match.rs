use super::{connection, users, User};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, QueryableByName, PartialEq)]
struct CountType {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub count: i64,
}

#[tokio::test]
async fn test_boxed_sql() {
    use diesel::query_builder::BoxedSqlQuery;

    let mut con = connection().await;

    let boxed: BoxedSqlQuery<_, _> =
        diesel::sql_query("select count(*) as count from users").into_boxed();

    let result = boxed.get_result::<CountType>(&mut con).await.unwrap();

    assert_eq!(result.count, 0);
}

#[tokio::test]
async fn test_select_statement() {
    use diesel::query_builder::SelectStatement;

    let mut con = connection().await;

    let select: SelectStatement<_, _, _, _, _, _, _> = users::table.select(User::as_select());

    let result = select.get_results(&mut con).await.unwrap();

    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_sql_query() {
    use diesel::query_builder::SqlQuery;

    let mut con = connection().await;

    let sql: SqlQuery<_> = diesel::sql_query("select count(*) as count from users");

    let result: CountType = sql.get_result(&mut con).await.unwrap();

    assert_eq!(result.count, 0);
}

#[tokio::test]
async fn test_unchecked_bind() {
    use diesel::expression::UncheckedBind;
    use diesel::sql_types::{BigInt, Text};

    let mut con = connection().await;

    let unchecked: UncheckedBind<_, _> =
        diesel::dsl::sql::<BigInt>("SELECT count(id) FROM users WHERE name = ")
            .bind::<Text, _>("Bob");

    let result = unchecked.get_result(&mut con).await;

    assert_eq!(Ok(0), result);
}

#[tokio::test]
async fn test_alias() {
    use diesel::query_source::Alias;

    let mut con = connection().await;

    let aliased: Alias<_> = diesel::alias!(users as other);

    let result = aliased.get_results::<User>(&mut con).await.unwrap();

    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_boxed_select() {
    use diesel::query_builder::BoxedSelectStatement;

    let mut con = connection().await;

    let select: BoxedSelectStatement<_, _, _, _> =
        users::table.select(User::as_select()).into_boxed();

    let result = select.get_results(&mut con).await.unwrap();

    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_sql_literal() {
    use diesel::expression::SqlLiteral;
    use diesel::sql_types::Integer;

    let mut con = connection().await;

    let literal: SqlLiteral<_> = diesel::dsl::sql::<Integer>("SELECT 6");

    let result = literal.get_result(&mut con).await;

    assert_eq!(Ok(6), result);
}

#[tokio::test]
async fn test_delete() {
    use diesel::query_builder::DeleteStatement;

    let mut con = connection().await;

    let delete: DeleteStatement<_, _, _> = diesel::delete(users::table);

    let result = delete.execute(&mut con).await;

    assert_eq!(Ok(0), result);
}

#[tokio::test]
async fn test_insert_statement() {
    use diesel::query_builder::InsertStatement;

    let mut con = connection().await;

    let inserted_names: InsertStatement<_, _, _, _> =
        diesel::insert_into(users::table).values(users::name.eq("Timmy"));

    let result = inserted_names.execute(&mut con).await;

    assert_eq!(Ok(1), result);
}

#[tokio::test]
async fn test_update_statement() {
    use diesel::query_builder::UpdateStatement;

    let mut con = connection().await;

    let update: UpdateStatement<_, _, _, _> = diesel::update(users::table)
        .set(users::name.eq("Jim"))
        .filter(users::name.eq("Sean"));

    let result = update.execute(&mut con).await;

    assert_eq!(Ok(0), result);
}
