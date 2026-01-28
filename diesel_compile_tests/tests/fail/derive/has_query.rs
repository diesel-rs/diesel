use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        author -> Integer,
        title -> Text,
    }
}

#[derive(HasQuery)]
struct User1 {
    //~^ ERROR: failed to resolve: use of unresolved module or unlinked crate `user1s`
    id: i32,
    name: String,
}

#[derive(HasQuery)]
#[diesel(table_name = posts)]
struct UserMixedUp {
    id: i32,
    name: String,
    //~^ ERROR: cannot find type `name` in module `posts`
    //~| ERROR: cannot find value `name` in module `posts`
}

#[derive(HasQuery)]
#[diesel(table_name = users)]
struct TypeMismatch {
    id: String,
    //~^ ERROR: the trait bound `std::string::String: FromSqlRow<diesel::sql_types::Integer, Mysql>` is not satisfied
    //~| ERROR: the trait bound `String: FromSqlRow<Integer, Sqlite>` is not satisfied
    //~| ERROR: the trait bound `std::string::String: FromSqlRow<diesel::sql_types::Integer, Pg>` is not satisfied
    name: i32,
    //~^ ERROR: the trait bound `i32: FromSqlRow<diesel::sql_types::Text, Mysql>` is not satisfied
    //~| ERROR: the trait bound `i32: FromSqlRow<diesel::sql_types::Text, Sqlite>` is not satisfied
    //~| ERROR: the trait bound `i32: FromSqlRow<diesel::sql_types::Text, Pg>` is not satisfied
}

#[derive(HasQuery)]
//~^ ERROR: the trait bound `SelectStatement<FromClause<table>>: SelectDsl<...>` is not satisfied
//~| ERROR: the trait bound `users::table: TableNotEqual<posts::table>` is not satisfied
//~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
#[diesel(table_name = users)]
struct RequiresValidSelect {
    #[diesel(select_expression = posts::id)]
    id: i32,
}

#[derive(HasQuery)]
//~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ...>: SelectDsl<...>` is not satisfied
//~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
//~| ERROR: the trait bound `users::table: TableNotEqual<posts::table>` is not satisfied
#[diesel(base_query = users::table.filter(users::id.eq(42_i32)))]
struct BaseQueryStillRequiresValidSelect {
    #[diesel(select_expression = posts::id)]
    id: i32,
}

#[derive(HasQuery)]
//~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ...>: SelectDsl<...>` is not satisfied
//~| ERROR: type mismatch resolving `<name as IsContainedInGroupBy<id>>::Output == Yes`
#[diesel(base_query = users::table.group_by(users::name))]
#[diesel(table_name = users)]
struct GroupByIsRespected {
    id: i32,
}
