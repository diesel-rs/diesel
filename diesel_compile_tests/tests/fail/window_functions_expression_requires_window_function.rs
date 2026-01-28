extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

fn main() {
    use diesel::dsl::*;

    users::table.select(lower(users::name).partition_by(users::id));
    //~^ ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`
    //~| ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`

    users::table.select(lower(users::name).over());
    //~^ ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`
    //~| ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`

    users::table.select(lower(users::name).window_filter(users::id.eq(42)));
    //~^ ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`
    //~| ERROR: the trait bound `lower<Text, name>: IsAggregateFunction` is not satisfied
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`

    users::table.select(lower(users::name).window_order(users::id));
    //~^ ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`
    //~| ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`

    users::table
        .select(lower(users::name).frame_by(frame::Rows.frame_start_with(frame::CurrentRow)));
    //~^ ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`
    //~| ERROR: `lower<Text, name>` is not a window function
    //~| ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`
}
