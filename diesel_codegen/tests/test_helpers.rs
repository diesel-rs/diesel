use diesel::prelude::*;

pub type TestConnection = SqliteConnection;

pub fn connection() -> TestConnection {
    SqliteConnection::establish(":memory:").unwrap()
}
