use diesel::prelude::*;

table! {
    users(id) {
        id -> Integer,
        name -> Nullable<Text>,
    }
}

#[derive(Insertable)]
struct User {
    id: String,
    #[diesel(embed, treat_none_as_default_value = true)]
    //~^ ERROR: `embed` and `treat_none_as_default_value` are mutually exclusive
    name: Option<i32>,
}

fn main() {}
