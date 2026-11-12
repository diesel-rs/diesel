#[cfg(feature = "postgres")]
#[tokio::test]
async fn notifications_arrive() {
    use diesel_async::RunQueryDsl;
    use futures_util::{StreamExt, TryStreamExt};

    let conn = &mut super::connection_without_transaction().await;

    diesel::sql_query("LISTEN test_notifications")
        .execute(conn)
        .await
        .unwrap();

    diesel::sql_query("NOTIFY test_notifications, 'first'")
        .execute(conn)
        .await
        .unwrap();

    diesel::sql_query("NOTIFY test_notifications, 'second'")
        .execute(conn)
        .await
        .unwrap();

    let notifications = conn
        .notifications_stream()
        .take(2)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    assert_eq!(2, notifications.len());
    assert_eq!(notifications[0].channel, "test_notifications");
    assert_eq!(notifications[1].channel, "test_notifications");
    assert_eq!(notifications[0].payload, "first");
    assert_eq!(notifications[1].payload, "second");

    let next_notification = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        std::pin::pin!(conn.notifications_stream()).next(),
    )
    .await;

    assert!(
        next_notification.is_err(),
        "Got a next notification, while not expecting one: {next_notification:?}"
    );

    diesel::sql_query("NOTIFY test_notifications")
        .execute(conn)
        .await
        .unwrap();

    let next_notification = std::pin::pin!(conn.notifications_stream()).next().await;
    assert_eq!(next_notification.unwrap().unwrap().payload, "");
}
