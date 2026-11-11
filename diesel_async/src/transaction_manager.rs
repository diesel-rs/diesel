use diesel::connection::InstrumentationEvent;
use diesel::connection::TransactionManagerStatus;
use diesel::connection::{
    InTransactionStatus, TransactionDepthChange, ValidTransactionManagerStatus,
};
use diesel::result::Error;
use diesel::QueryResult;
use std::borrow::Cow;
use std::future::Future;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A helper trait to allow us asserting additional bounds on `AsyncFnOnce`
/// Especially this lets us assert bounds on the future returned by the closure
/// while still maintaining working type inference for the closure
///
/// This mostly exists for the following reasons:
///
/// * `AsyncFnOnce::CallOnceFuture` is not stable, so you cannot assert only with `AsyncFnOnce` that
///   the returned future is `Send`
/// * Only using `FnOnce(T) -> impl Future` doesn't work as you cannot use `impl Future` in that position
/// * Using `F: FnOnce(T) -> Fut, Fut: Future<…>` doesn't work as you run into diverging lifetimes,
///   as you essentially need to reuse the higher ranked lifetime from the closure to also restrict
///   the lifetime of the returned future
/// * Using a `trait Func<T>: FnOnce(T) -> <Self as Func<T>::Output> { type Output }` allows us
///   to restrict the lifetime and bounds (like `Future` and `Send`) on the output type of the closure
///   but then fails in type inference due to rustc bugs. You get unhelpful errors like
///   `implementation of `FnOnce` is not general enough`, this can be side stepped by putting
///   types on the calling side of the closure, which is not that nice API wise
///
/// This workaround is still not optimal as it still requires us to have this trait and show
/// it as public bound. We somewhat try to avoid confusing users there by having an additional
/// `AsyncFnOnce(T) -> R` bound at the calling side that hopefully will show up in rustdoc
/// as well and hopefully guides the user to do the "right thing"
pub trait AsyncFunc<T, R>:
    AsyncFnOnce(T) -> R + FnOnce(T) -> <Self as AsyncFunc<T, R>>::Fut
{
    type Fut: Future<Output = R>;
}

impl<F, T, Fut, R> AsyncFunc<T, R> for F
where
    F: AsyncFnOnce(T) -> R + FnOnce(T) -> Fut,
    Fut: Future<Output = R>,
{
    type Fut = Fut;
}

use crate::AsyncConnection;
// TODO: refactor this to share more code with diesel

/// Manages the internal transaction state for a connection.
///
/// You will not need to interact with this trait, unless you are writing an
/// implementation of [`AsyncConnection`].
pub trait TransactionManager<Conn: AsyncConnection>: Send {
    /// Data stored as part of the connection implementation
    /// to track the current transaction state of a connection
    type TransactionStateData;

    /// Begin a new transaction or savepoint
    ///
    /// If the transaction depth is greater than 0,
    /// this should create a savepoint instead.
    /// This function is expected to increment the transaction depth by 1.
    fn begin_transaction(conn: &mut Conn) -> impl Future<Output = QueryResult<()>> + Send;

    /// Rollback the inner-most transaction or savepoint
    ///
    /// If the transaction depth is greater than 1,
    /// this should rollback to the most recent savepoint.
    /// This function is expected to decrement the transaction depth by 1.
    fn rollback_transaction(conn: &mut Conn) -> impl Future<Output = QueryResult<()>> + Send;

    /// Commit the inner-most transaction or savepoint
    ///
    /// If the transaction depth is greater than 1,
    /// this should release the most recent savepoint.
    /// This function is expected to decrement the transaction depth by 1.
    fn commit_transaction(conn: &mut Conn) -> impl Future<Output = QueryResult<()>> + Send;

    /// Fetch the current transaction status as mutable
    ///
    /// Used to ensure that `begin_test_transaction` is not called when already
    /// inside of a transaction, and that operations are not run in a `InError`
    /// transaction manager.
    #[doc(hidden)]
    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut TransactionManagerStatus;

    /// Executes the given function inside of a database transaction
    ///
    /// Each implementation of this function needs to fulfill the documented
    /// behaviour of [`AsyncConnection::transaction`]
    fn transaction<'a, 'conn, F, R, E>(
        conn: &'conn mut Conn,
        callback: F,
    ) -> impl Future<Output = Result<R, E>> + Send + 'conn
    where
        for<'r> F: AsyncFnOnce(&'r mut Conn) -> Result<R, E>
            + AsyncFunc<&'r mut Conn, Result<R, E>, Fut: Send>
            + Send
            + 'a,
        E: From<Error> + Send,
        R: Send,
        'a: 'conn,
    {
        async move {
            let callback = callback;

            Self::begin_transaction(conn).await?;
            match callback(&mut *conn).await {
                Ok(value) => {
                    Self::commit_transaction(conn).await?;
                    Ok(value)
                }
                Err(user_error) => match Self::rollback_transaction(conn).await {
                    Ok(()) => Err(user_error),
                    Err(Error::BrokenTransactionManager) => {
                        // In this case we are probably more interested by the
                        // original error, which likely caused this
                        Err(user_error)
                    }
                    Err(rollback_error) => Err(rollback_error.into()),
                },
            }
        }
    }

    /// This methods checks if the connection manager is considered to be broken
    /// by connection pool implementations
    ///
    /// A connection manager is considered to be broken by default if it either
    /// contains an open transaction (because you don't want to have connections
    /// with open transactions in your pool) or when the transaction manager is
    /// in an error state.
    #[doc(hidden)]
    fn is_broken_transaction_manager(conn: &mut Conn) -> bool {
        check_broken_transaction_state(conn)
    }
}

fn check_broken_transaction_state<Conn>(conn: &mut Conn) -> bool
where
    Conn: AsyncConnection,
{
    match Conn::TransactionManager::transaction_manager_status_mut(conn).transaction_state() {
        // all transactions are closed
        // so we don't consider this connection broken
        Ok(ValidTransactionManagerStatus {
            in_transaction: None,
            ..
        }) => false,
        // The transaction manager is in an error state
        // Therefore we consider this connection broken
        Err(_) => true,
        // The transaction manager contains a open transaction
        // we do consider this connection broken
        // if that transaction was not opened by `begin_test_transaction`
        Ok(ValidTransactionManagerStatus {
            in_transaction: Some(s),
            ..
        }) => !s.test_transaction,
    }
}

/// An implementation of `TransactionManager` which can be used for backends
/// which use ANSI standard syntax for savepoints such as SQLite and PostgreSQL.
#[derive(Default, Debug)]
pub struct AnsiTransactionManager {
    pub(crate) status: TransactionManagerStatus,
    // this boolean flag tracks whether we are currently in the process
    // of executing any transaction related SQL (BEGIN, COMMIT, ROLLBACK)
    // if we ever encounter a situation where this flag is set
    // while the connection is returned to a pool
    // that means the connection is broken as someone dropped the
    // transaction future while these commands where executed
    // and we cannot know the connection state anymore
    //
    // We ensure this by wrapping all calls to `.await`
    // into `AnsiTransactionManager::critical_transaction_block`
    // below
    //
    // See https://github.com/weiznich/diesel_async/issues/198 for
    // details
    pub(crate) is_broken: Arc<AtomicBool>,
    // this boolean flag tracks whether we are currently in this process
    // of trying to commit the transaction. this is useful because if we
    // are and we get a serialization failure, we might not want to attempt
    // a rollback up the chain.
    pub(crate) is_commit: bool,
}

impl AnsiTransactionManager {
    fn get_transaction_state<Conn>(
        conn: &mut Conn,
    ) -> QueryResult<&mut ValidTransactionManagerStatus>
    where
        Conn: AsyncConnection<TransactionManager = Self>,
    {
        conn.transaction_state().status.transaction_state()
    }

    /// Begin a transaction with custom SQL
    ///
    /// This is used by connections to implement more complex transaction APIs
    /// to set things such as isolation levels.
    /// Returns an error if already inside of a transaction.
    pub async fn begin_transaction_sql<Conn>(conn: &mut Conn, sql: &str) -> QueryResult<()>
    where
        Conn: AsyncConnection<TransactionManager = Self>,
    {
        let is_broken = conn.transaction_state().is_broken.clone();
        let state = Self::get_transaction_state(conn)?;
        if let Some(_depth) = state.transaction_depth() {
            return Err(Error::AlreadyInTransaction);
        }
        let instrumentation_depth = NonZeroU32::new(1);

        conn.instrumentation()
            .on_connection_event(InstrumentationEvent::begin_transaction(
                instrumentation_depth.expect("We know that 1 is not zero"),
            ));

        // Keep remainder of this method in sync with `begin_transaction()`.
        Self::critical_transaction_block(&is_broken, conn.batch_execute(sql)).await?;
        Self::get_transaction_state(conn)?
            .change_transaction_depth(TransactionDepthChange::IncreaseDepth)?;
        Ok(())
    }

    // This function should be used to await any connection
    // related future in our transaction manager implementation
    //
    // It takes care of tracking entering and exiting executing the future
    // which in turn is used to determine if it's safe to still use
    // the connection in the event of a canceled transaction execution
    async fn critical_transaction_block<F>(is_broken: &AtomicBool, f: F) -> F::Output
    where
        F: std::future::Future,
    {
        let was_broken = is_broken.swap(true, Ordering::Relaxed);
        debug_assert!(
            !was_broken,
            "Tried to execute a transaction SQL on transaction manager that was previously cancled"
        );
        let res = f.await;
        is_broken.store(false, Ordering::Relaxed);
        res
    }
}

impl<Conn> TransactionManager<Conn> for AnsiTransactionManager
where
    Conn: AsyncConnection<TransactionManager = Self>,
{
    type TransactionStateData = Self;

    async fn begin_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;
        let start_transaction_sql = match transaction_state.transaction_depth() {
            None => Cow::from("BEGIN"),
            Some(transaction_depth) => {
                Cow::from(format!("SAVEPOINT diesel_savepoint_{transaction_depth}"))
            }
        };
        let depth = transaction_state
            .transaction_depth()
            .and_then(|d| d.checked_add(1))
            .unwrap_or(NonZeroU32::new(1).expect("It's not 0"));
        conn.instrumentation()
            .on_connection_event(InstrumentationEvent::begin_transaction(depth));
        Self::critical_transaction_block(
            &conn.transaction_state().is_broken.clone(),
            conn.batch_execute(&start_transaction_sql),
        )
        .await?;
        Self::get_transaction_state(conn)?
            .change_transaction_depth(TransactionDepthChange::IncreaseDepth)?;

        Ok(())
    }

    async fn rollback_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;

        let (
            (rollback_sql, rolling_back_top_level),
            requires_rollback_maybe_up_to_top_level_before_execute,
        ) = match transaction_state.in_transaction {
            Some(ref in_transaction) => (
                match in_transaction.transaction_depth.get() {
                    1 => (Cow::Borrowed("ROLLBACK"), true),
                    depth_gt1 => (
                        Cow::Owned(format!(
                            "ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                            depth_gt1 - 1
                        )),
                        false,
                    ),
                },
                in_transaction.requires_rollback_maybe_up_to_top_level,
            ),
            None => return Err(Error::NotInTransaction),
        };

        let depth = transaction_state
            .transaction_depth()
            .expect("We know that we are in a transaction here");
        conn.instrumentation()
            .on_connection_event(InstrumentationEvent::rollback_transaction(depth));

        let is_broken = conn.transaction_state().is_broken.clone();

        match Self::critical_transaction_block(&is_broken, conn.batch_execute(&rollback_sql)).await
        {
            Ok(()) => {
                match Self::get_transaction_state(conn)?
                    .change_transaction_depth(TransactionDepthChange::DecreaseDepth)
                {
                    Ok(()) => {}
                    Err(Error::NotInTransaction) if rolling_back_top_level => {
                        // Transaction exit may have already been detected by connection
                        // implementation. It's fine.
                    }
                    Err(e) => return Err(e),
                }
                Ok(())
            }
            Err(rollback_error) => {
                let tm_status = Self::transaction_manager_status_mut(conn);
                match tm_status {
                    TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
                        in_transaction:
                            Some(InTransactionStatus {
                                transaction_depth,
                                requires_rollback_maybe_up_to_top_level,
                                ..
                            }),
                        ..
                    }) if transaction_depth.get() > 1 => {
                        // A savepoint failed to rollback - we may still attempt to repair
                        // the connection by rolling back higher levels.

                        // To make it easier on the user (that they don't have to really
                        // look at actual transaction depth and can just rely on the number
                        // of times they have called begin/commit/rollback) we still
                        // decrement here:
                        *transaction_depth = NonZeroU32::new(transaction_depth.get() - 1)
                            .expect("Depth was checked to be > 1");
                        *requires_rollback_maybe_up_to_top_level = true;
                        if requires_rollback_maybe_up_to_top_level_before_execute {
                            // In that case, we tolerate that savepoint releases fail
                            // -> we should ignore errors
                            return Ok(());
                        }
                    }
                    TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
                        in_transaction: None,
                        ..
                    }) => {
                        // we would have returned `NotInTransaction` if that was already the state
                        // before we made our call
                        // => Transaction manager status has been fixed by the underlying connection
                        // so we don't need to set_in_error
                    }
                    _ => tm_status.set_in_error(),
                }
                Err(rollback_error)
            }
        }
    }

    /// If the transaction fails to commit due to a `SerializationFailure` or a
    /// `ReadOnlyTransaction` a rollback will be attempted. If the rollback succeeds,
    /// the original error will be returned, otherwise the error generated by the rollback
    /// will be returned. In the second case the connection will be considered broken
    /// as it contains a uncommitted unabortable open transaction.
    async fn commit_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;
        let transaction_depth = transaction_state.transaction_depth();
        let (commit_sql, committing_top_level) = match transaction_depth {
            None => return Err(Error::NotInTransaction),
            Some(transaction_depth) if transaction_depth.get() == 1 => {
                (Cow::Borrowed("COMMIT"), true)
            }
            Some(transaction_depth) => (
                Cow::Owned(format!(
                    "RELEASE SAVEPOINT diesel_savepoint_{}",
                    transaction_depth.get() - 1
                )),
                false,
            ),
        };
        let depth = transaction_state
            .transaction_depth()
            .expect("We know that we are in a transaction here");
        conn.instrumentation()
            .on_connection_event(InstrumentationEvent::commit_transaction(depth));

        let is_broken = {
            let transaction_state = conn.transaction_state();
            transaction_state.is_commit = true;
            transaction_state.is_broken.clone()
        };

        let res =
            Self::critical_transaction_block(&is_broken, conn.batch_execute(&commit_sql)).await;

        conn.transaction_state().is_commit = false;

        match res {
            Ok(()) => {
                match Self::get_transaction_state(conn)?
                    .change_transaction_depth(TransactionDepthChange::DecreaseDepth)
                {
                    Ok(()) => {}
                    Err(Error::NotInTransaction) if committing_top_level => {
                        // Transaction exit may have already been detected by connection.
                        // It's fine
                    }
                    Err(e) => return Err(e),
                }
                Ok(())
            }
            Err(commit_error) => {
                if let TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
                    in_transaction:
                        Some(InTransactionStatus {
                            requires_rollback_maybe_up_to_top_level: true,
                            ..
                        }),
                    ..
                }) = conn.transaction_state().status
                {
                    // rollback_transaction handles the critical block internally on its own
                    match Self::rollback_transaction(conn).await {
                        Ok(()) => {}
                        Err(rollback_error) => {
                            conn.transaction_state().status.set_in_error();
                            return Err(Error::RollbackErrorOnCommit {
                                rollback_error: Box::new(rollback_error),
                                commit_error: Box::new(commit_error),
                            });
                        }
                    }
                } else {
                    Self::get_transaction_state(conn)?
                        .change_transaction_depth(TransactionDepthChange::DecreaseDepth)?;
                }
                Err(commit_error)
            }
        }
    }

    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut TransactionManagerStatus {
        &mut conn.transaction_state().status
    }

    fn is_broken_transaction_manager(conn: &mut Conn) -> bool {
        conn.transaction_state().is_broken.load(Ordering::Relaxed)
            || check_broken_transaction_state(conn)
    }
}
