#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen, dotenv_macros)]

#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::*;
use dotenv::dotenv;

table! {
    users {
        id -> Serial,
        name -> VarChar,
        favorite_color -> Nullable<VarChar>,
    }
}

#[derive(Debug, Queriable)]
#[changeset_for(users)]
struct User {
    id: i32,
    name: String,
    favorite_color: Option<String>,
}

#[derive(Debug, Queriable)]
#[insertable_into(users)]
#[changeset_for(users)]
struct NewUser {
    pub name: String,
    pub favorite_color: Option<String>,
}

impl NewUser {
    pub fn new(name: &str, favorite_color: Option<&str>) -> Self {
        NewUser {
            name: name.to_string(),
            favorite_color: favorite_color.map(|s| s.to_string()),
        }
    }
}

fn main() {
    use users::dsl::*;
    dotenv().ok();

    let connection = connection();
    setup_users_table(&connection);
    let data: &[_] = &[NewUser::new("Sean", None),
                       NewUser::new("Tess", None),
                       NewUser::new("Bob", Some("Blue"))];
    diesel::query_builder::insert(data).into(users).execute(&connection).unwrap();

    let mut user: User = connection.find(users, 3).unwrap();
    user.favorite_color = Some("Orange".to_owned());
    match user.save_changes::<User>(&connection) {
        Ok(user) => {
            println!("{}'s favorite_color is {}",
                     user.name,
                     user.favorite_color.unwrap())
        }
        Err(err) => println!("{}", err),
    }

    let all_users: Vec<User> = users.load(&connection)
                                    .unwrap()
                                    .collect();

    println!("Here are all the users in our database: {:?}", all_users);
}

fn setup_users_table(connection: &Connection) {
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        favorite_color VARCHAR
    )").unwrap();
}

fn connection() -> Connection {
    let result = connection_without_transaction();
    result.begin_test_transaction().unwrap();
    result
}

fn connection_without_transaction() -> Connection {
    let connection_url = dotenv!("DATABASE_URL",
                                 "DATABASE_URL must be set in order to run example");
    Connection::establish(&connection_url).unwrap()
}
