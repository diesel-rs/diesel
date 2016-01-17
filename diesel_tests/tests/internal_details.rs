use schema::*;
use diesel::types::*;
use diesel::expression::AsExpression;
use diesel::expression::dsl::sql;
use diesel::*;

#[test]
fn bind_params_are_passed_for_null_when_not_inserting() {
    let connection = connection();
    let query = select(sql::<Integer>("1"))
        .filter(AsExpression::<Nullable<Integer>>::as_expression(None::<i32>).is_null());
    assert_eq!(Ok(1), query.first(&connection));
}
