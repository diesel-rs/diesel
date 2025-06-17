#[macro_use]
extern crate diesel;

#[derive(Queryable)]
#[diesel(what = true)]
//~^ ERROR: unknown attribute, expected one of `aggregate`, `not_sized`, `foreign_derive`, `table_name`, `sql_type`, `treat_none_as_default_value`, `treat_none_as_null`, `belongs_to`, `mysql_type`, `sqlite_type`, `postgres_type`, `primary_key`, `check_for_backend`
struct User1 {
    id: i32,
}

#[derive(Queryable)]
struct User2 {
    #[diesel(what = true)]
    //~^ ERROR: unknown attribute, expected one of `embed`, `skip_insertion`, `column_name`, `sql_type`, `treat_none_as_default_value`, `treat_none_as_null`, `serialize_as`, `deserialize_as`, `select_expression`, `select_expression_type`
    id: i32,
}

fn main() {}
