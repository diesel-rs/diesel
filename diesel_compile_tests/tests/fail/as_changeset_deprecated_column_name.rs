#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[table_name(users)]
struct UserForm {
    id: i32,
    #[column_name(name)]
    name: String,
}

fn main() {
    // Workaround for https://github.com/dtolnay/trybuild/issues/8
    compile_error!("warnings");
}
