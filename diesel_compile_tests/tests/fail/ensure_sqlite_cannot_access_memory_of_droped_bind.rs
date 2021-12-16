use diesel::prelude::*;
use diesel::sql_types;

fn main() {
    {
        let mut connection = SqliteConnection::establish("").unwrap();

        let buf: Vec<u8> = vec![0, 1, 2];

        let query = diesel::select((&buf as &[u8]).into_sql::<sql_types::Binary>());

        let mut iter = Connection::load(&mut connection, query).unwrap();

        // Sqlite borrows the buffer internally, so droping it here is not allowed
        // while the statement is still alive.
        std::mem::drop(buf);

        assert_eq!(iter.next().is_some(), true);
        assert_eq!(iter.next().is_none(), true);
    }

    // Everything else is allowed
    {
        let mut connection = PgConnection::establish("").unwrap();

        let buf: Vec<u8> = vec![0, 1, 2];

        let query = diesel::select((&buf as &[u8]).into_sql::<sql_types::Binary>());

        let mut iter = Connection::load(&mut connection, query).unwrap();

        std::mem::drop(buf);

        assert_eq!(iter.next().is_some(), true);
        assert_eq!(iter.next().is_none(), true);
    }

    {
        let mut connection = MysqlConnection::establish("").unwrap();

        let buf: Vec<u8> = vec![0, 1, 2];

        let query = diesel::select((&buf as &[u8]).into_sql::<sql_types::Binary>());

        let mut iter = Connection::load(&mut connection, query).unwrap();

        std::mem::drop(buf);

        assert_eq!(iter.next().is_some(), true);
        assert_eq!(iter.next().is_none(), true);
    }

    {
        let mut connection = SqliteConnection::establish("").unwrap();

        let buf: Vec<u8> = vec![0, 1, 2];

        let query = diesel::select((&buf as &[u8]).into_sql::<sql_types::Binary>());

        let mut iter = Connection::load(&mut connection, query).unwrap();

        assert_eq!(iter.next().is_some(), true);
        assert_eq!(iter.next().is_none(), true);

        std::mem::drop(iter);
        std::mem::drop(buf);
    }

}
