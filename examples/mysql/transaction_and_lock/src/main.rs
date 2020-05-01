#[macro_use]
extern crate diesel;

use diesel::prelude::*;
// use diesel::query_dsl::methods::{LockingDsl};

pub fn establish_connection() -> MysqlConnection {
    dotenv::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    MysqlConnection::establish(&database_url).unwrap()
}

mod schema {
    table! {
        users {
            id -> Unsigned<Integer>,
            account_balance -> Integer,
        }
    }
}

use schema::users;
use schema::users::dsl::users as users_dsl;

#[derive(Queryable, Debug)]
pub struct User {
    pub id: u32,
    pub account_balance: i32
}

fn create_user(
    ref_db_connection: &MysqlConnection,
) -> User {
    diesel::insert_into(users::table)
      .default_values()
      .execute(ref_db_connection)
      .unwrap();

    users_dsl.order(users::id.desc()).first(ref_db_connection).unwrap()
}

fn update_user_balance_with_lock(
    ref_db_connection: &MysqlConnection,
    user_id: u32,
    new_balance: i32,
) {
    ref_db_connection.transaction::<_, diesel::result::Error, _>(|| {
        // Lock the user record to avoid modification by other threads
        users_dsl.find(user_id).for_update().execute(ref_db_connection)?;

        // Error: diesel::update(users_dsl.find(user_id).with_lock())
        diesel::update(users_dsl.find(user_id))
          .set(users::account_balance.eq(new_balance))
          .execute(ref_db_connection)?;
        Ok(())
    }).unwrap();
}

fn main() {
    let db_connection = establish_connection();
    let user = create_user(&db_connection);
    let new_balance: i32 = 10;
    update_user_balance_with_lock(&db_connection, user.id, new_balance);
    let user_new_state: User = users_dsl.find(user.id).first(&db_connection).unwrap();
    assert_eq!(user_new_state.account_balance, new_balance);
    dbg!(user_new_state);
}
