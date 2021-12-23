extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    users.for_update().distinct();
    users.distinct().for_update();
    users.for_update().distinct_on(id);
    users.distinct_on(id).for_update();

    users.for_update().group_by(id);
    users.group_by(id).for_update();

    users.into_boxed().for_update();
    users.for_update().into_boxed();

    users.for_update().group_by(id).having(id.gt(1));
    users.group_by(id).having(id.gt(1)).for_update();
}
