extern crate diesel;

use diesel::dsl::count;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    name: String,
}

fn main() {
    use self::users::dsl::*;

    let stmt = update(users.filter(id.eq(1)))
        .set(name.eq("Bill"))
        .returning(count(id));
    //~^ ERROR: the trait bound `diesel::expression::is_aggregate::Yes: MixedAggregates<diesel::expression::is_aggregate::No>` is not satisfied

    let new_user = NewUser {
        name: "Foobar".to_string(),
    };
    let stmt = insert_into(users)
        .values(&new_user)
        .returning((name, count(name)));
    //~^ ERROR: the trait bound `diesel::expression::is_aggregate::No: MixedAggregates<diesel::expression::is_aggregate::Yes>` is not satisfied
}
