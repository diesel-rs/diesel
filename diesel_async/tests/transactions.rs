#[cfg(feature = "postgres")]
#[tokio::test]
async fn concurrent_serializable_transactions_behave_correctly() {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    table! {
        users3 {
            id -> Integer,
        }
    }

    // create an async connection
    let mut conn = super::connection_without_transaction().await;

    let mut conn1 = super::connection_without_transaction().await;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS users3 (id int);")
        .execute(&mut conn)
        .await
        .unwrap();

    let barrier_1 = Arc::new(Barrier::new(2));
    let barrier_2 = Arc::new(Barrier::new(2));
    let barrier_3 = Arc::new(Barrier::new(2));
    let barrier_1_for_tx1 = barrier_1.clone();
    let barrier_1_for_tx2 = barrier_1.clone();
    let barrier_2_for_tx1 = barrier_2.clone();
    let barrier_2_for_tx2 = barrier_2.clone();
    let barrier_3_for_tx1 = barrier_3.clone();
    let barrier_3_for_tx2 = barrier_3.clone();

    let mut tx = conn.build_transaction().serializable().read_write();

    let res = tx.run(async |conn| {
        users3::table.select(users3::id).load::<i32>(conn).await?;

        barrier_1_for_tx1.wait().await;
        diesel::insert_into(users3::table)
            .values(users3::id.eq(1))
            .execute(conn)
            .await?;
        barrier_3_for_tx1.wait().await;
        barrier_2_for_tx1.wait().await;

        Ok::<_, diesel::result::Error>(())
    });

    let mut tx1 = conn1.build_transaction().serializable().read_write();

    let res1 = async {
        let res = tx1
            .run(async |conn| {
                users3::table.select(users3::id).load::<i32>(conn).await?;

                barrier_1_for_tx2.wait().await;
                diesel::insert_into(users3::table)
                    .values(users3::id.eq(1))
                    .execute(conn)
                    .await?;
                barrier_3_for_tx2.wait().await;

                Ok::<_, diesel::result::Error>(())
            })
            .await;
        barrier_2_for_tx2.wait().await;
        res
    };

    let (res, res1) = tokio::join!(res, res1);
    let _ = diesel::sql_query("DROP TABLE users3")
        .execute(&mut conn1)
        .await;

    assert!(
        res1.is_ok(),
        "Expected the second transaction to be succussfull, but got an error: {:?}",
        res1.unwrap_err()
    );

    assert!(res.is_err(), "Expected the first transaction to fail");
    let err = res.unwrap_err();
    assert!(
        matches!(
            &err,
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::SerializationFailure,
                _
            )
        ),
        "Expected an serialization failure but got another error: {err:?}"
    );

    let mut tx = conn.build_transaction();

    let res = tx.run(async |_| Ok::<_, diesel::result::Error>(())).await;

    assert!(
        res.is_ok(),
        "Expect transaction to run fine but got an error: {:?}",
        res.unwrap_err()
    );
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn commit_with_serialization_failure_already_ends_transaction() {
    use diesel::prelude::*;
    use diesel_async::{AsyncConnection, RunQueryDsl};
    use std::sync::Arc;
    use tokio::sync::Barrier;

    table! {
        users4 {
            id -> Integer,
        }
    }

    // create an async connection
    let mut conn = super::connection_without_transaction().await;

    struct A(Vec<&'static str>);
    impl diesel::connection::Instrumentation for A {
        fn on_connection_event(&mut self, event: diesel::connection::InstrumentationEvent<'_>) {
            if let diesel::connection::InstrumentationEvent::StartQuery { query, .. } = event {
                let q = query.to_string();
                let q = q.split_once(' ').map(|(a, _)| a).unwrap_or(&q);

                if matches!(q, "BEGIN" | "COMMIT" | "ROLLBACK") {
                    assert_eq!(q, self.0.pop().unwrap());
                }
            }
        }
    }
    conn.set_instrumentation(A(vec!["COMMIT", "BEGIN", "COMMIT", "BEGIN"]));

    let mut conn1 = super::connection_without_transaction().await;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS users4 (id int);")
        .execute(&mut conn)
        .await
        .unwrap();

    let barrier_1 = Arc::new(Barrier::new(2));
    let barrier_2 = Arc::new(Barrier::new(2));
    let barrier_3 = Arc::new(Barrier::new(2));
    let barrier_1_for_tx1 = barrier_1.clone();
    let barrier_1_for_tx2 = barrier_1.clone();
    let barrier_2_for_tx1 = barrier_2.clone();
    let barrier_2_for_tx2 = barrier_2.clone();
    let barrier_3_for_tx1 = barrier_3.clone();
    let barrier_3_for_tx2 = barrier_3.clone();

    let mut tx = conn.build_transaction().serializable().read_write();

    let res = tx.run(async |conn| {
        users4::table.select(users4::id).load::<i32>(conn).await?;

        barrier_1_for_tx1.wait().await;
        diesel::insert_into(users4::table)
            .values(users4::id.eq(1))
            .execute(conn)
            .await?;
        barrier_3_for_tx1.wait().await;
        barrier_2_for_tx1.wait().await;

        Ok::<_, diesel::result::Error>(())
    });

    let mut tx1 = conn1.build_transaction().serializable().read_write();

    let res1 = async {
        let res = tx1
            .run(async |conn| {
                users4::table.select(users4::id).load::<i32>(conn).await?;

                barrier_1_for_tx2.wait().await;
                diesel::insert_into(users4::table)
                    .values(users4::id.eq(1))
                    .execute(conn)
                    .await?;
                barrier_3_for_tx2.wait().await;

                Ok::<_, diesel::result::Error>(())
            })
            .await;
        barrier_2_for_tx2.wait().await;
        res
    };

    let (res, res1) = tokio::join!(res, res1);
    let _ = diesel::sql_query("DROP TABLE users4")
        .execute(&mut conn1)
        .await;

    assert!(
        res1.is_ok(),
        "Expected the second transaction to be succussfull, but got an error: {:?}",
        res1.unwrap_err()
    );

    assert!(res.is_err(), "Expected the first transaction to fail");
    let err = res.unwrap_err();
    assert!(
        matches!(
            &err,
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::SerializationFailure,
                _
            )
        ),
        "Expected an serialization failure but got another error: {err:?}"
    );

    let mut tx = conn.build_transaction();

    let res = tx.run(async |_| Ok::<_, diesel::result::Error>(())).await;

    assert!(
        res.is_ok(),
        "Expect transaction to run fine but got an error: {:?}",
        res.unwrap_err()
    );
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn read_only_transaction_error_kind() {
    use assert_matches::assert_matches;
    use diesel::insert_into;
    use diesel::prelude::*;
    use diesel::result::Error;
    use diesel_async::RunQueryDsl;

    table! {
        use diesel::sql_types::*;
        users_read_only {
            id -> Integer,
        }
    }

    #[derive(Insertable, Debug)]
    #[diesel(table_name = users_read_only)]
    struct User {
        id: i32,
    }

    let mut conn = super::connection_without_transaction().await;
    diesel::sql_query("CREATE TABLE IF NOT EXISTS users_read_only ( id INTEGER PRIMARY KEY )")
        .execute(&mut conn)
        .await
        .unwrap();

    let mut tx = conn.build_transaction().read_only();

    let err = tx
        .run(async |c| {
            insert_into(users_read_only::table)
                .values(&[User { id: 1 }])
                .execute(c)
                .await?;
            Ok::<_, Error>(())
        })
        .await
        .unwrap_err();

    assert_matches!(err, Error::DatabaseError(kind, _) if kind == diesel::result::DatabaseErrorKind::ReadOnlyTransaction);
}
