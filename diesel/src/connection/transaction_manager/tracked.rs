//! Transaction frames of connections that report the transaction control
//! statements they complete, so that SQL text and the transaction manager
//! share one view of the open transaction and savepoints.

use super::{
    AnsiTransactionManager, InTransactionStatus, TransactionDepthChange, TransactionManagerStatus,
    ValidTransactionManagerStatus,
};
use crate::connection::Connection;
use crate::connection::instrumentation::InstrumentationEvent;
use crate::result::{Error, QueryResult};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::num::NonZeroU32;

/// The transaction and savepoints open on a connection, oldest first.
#[derive(Debug, Default)]
pub(crate) struct TransactionFrames {
    frames: Vec<TransactionFrame>,
    /// Scopes opened by the transaction manager and not yet closed, oldest first.
    scopes: Vec<ScopeId>,
    next_scope: u64,
    /// The scope whose frame the transaction manager is opening.
    opening: Option<ScopeId>,
}

#[derive(Debug)]
struct TransactionFrame {
    /// `None` for the transaction opened by `BEGIN`.
    name: Option<Box<str>>,
    /// The transaction manager scope that opened this frame.
    scope: Option<ScopeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScopeId(u64);

/// A transaction control statement that the database completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransactionControl {
    /// `BEGIN`, in any mode
    Begin,
    /// `COMMIT` or `END`
    Commit,
    /// `ROLLBACK` without a savepoint name
    Rollback,
    /// `SAVEPOINT name`
    Savepoint(Box<str>),
    /// `RELEASE name`
    Release(Box<str>),
    /// `ROLLBACK TO name`
    RollbackTo(Box<str>),
}

fn depth_of(frames: usize) -> Option<NonZeroU32> {
    NonZeroU32::new(u32::try_from(frames).expect("Transaction depth is too large"))
}

/// The depth of the frame at `index`.
fn frame_depth(index: usize) -> NonZeroU32 {
    depth_of(index.saturating_add(1)).expect("The depth of a frame is never zero")
}

fn quoted(name: &str) -> String {
    alloc::format!("\"{}\"", name.replace('"', "\"\""))
}

impl TransactionFrames {
    /// The newest frame named `name`, compared the way SQLite compares identifiers.
    fn position(&self, name: &str) -> Option<usize> {
        self.frames.iter().rposition(|frame| {
            frame
                .name
                .as_deref()
                .is_some_and(|frame_name| frame_name.eq_ignore_ascii_case(name))
        })
    }

    fn scope_position(&self, scope: ScopeId) -> Option<usize> {
        self.frames
            .iter()
            .rposition(|frame| frame.scope == Some(scope))
    }

    /// The first `diesel_savepoint_N`, starting at the current depth, that no open savepoint uses.
    fn generated_name(&self) -> String {
        (self.frames.len()..)
            .map(|depth| alloc::format!("diesel_savepoint_{depth}"))
            .find(|name| self.position(name).is_none())
            .expect("The number of open savepoints is finite")
    }

    /// The SQL spelling of the name of the frame at `index`, which is not the outermost.
    fn sql_name(&self, index: usize) -> Cow<'_, str> {
        let frame = &self.frames[index];
        let name = frame
            .name
            .as_deref()
            .expect("Only the outermost frame can lack a name");
        if frame.scope.is_some() {
            // `generated_name` spells names that need no quoting.
            Cow::Borrowed(name)
        } else {
            Cow::Owned(quoted(name))
        }
    }

    /// The frame closed by the next commit or rollback. That call closes the
    /// newest open scope even if its frame has already ended.
    fn close_target(&mut self) -> QueryResult<usize> {
        match self.scopes.pop() {
            Some(scope) => self.scope_position(scope).ok_or(Error::NotInTransaction),
            None => self
                .frames
                .len()
                .checked_sub(1)
                .ok_or(Error::NotInTransaction),
        }
    }

    fn complete(&mut self, control: &TransactionControl, autocommit: bool) {
        match control {
            TransactionControl::Begin => {
                self.frames.clear();
                self.push(None);
            }
            // The statement stays cached with its name, so the frame needs its own copy.
            TransactionControl::Savepoint(name) => self.push(Some(name.clone())),
            TransactionControl::Commit | TransactionControl::Rollback => self.frames.clear(),
            TransactionControl::Release(name) => {
                if let Some(index) = self.position(name) {
                    self.frames.truncate(index);
                }
            }
            TransactionControl::RollbackTo(name) => {
                if let Some(index) = self.position(name) {
                    self.frames.truncate(index + 1);
                }
            }
        }
        if autocommit {
            self.frames.clear();
        }
    }

    fn push(&mut self, name: Option<Box<str>>) {
        let scope = self.opening.take();
        self.frames.push(TransactionFrame { name, scope });
    }
}

impl AnsiTransactionManager {
    /// A transaction manager for a connection that reports every transaction
    /// control statement it completes through [`Self::complete_control`].
    pub(crate) fn tracking_control_statements() -> Self {
        Self {
            status: TransactionManagerStatus::default(),
            frames: Some(TransactionFrames::default()),
        }
    }

    /// Records a transaction control statement the database completed.
    ///
    /// `autocommit` is whether the connection is in autocommit mode afterwards.
    pub(crate) fn complete_control(&mut self, control: &TransactionControl, autocommit: bool) {
        let Some(frames) = &mut self.frames else {
            return;
        };
        frames.complete(control, autocommit);
        let depth = depth_of(frames.frames.len());
        let TransactionManagerStatus::Valid(status) = &mut self.status else {
            return;
        };
        match (depth, &mut status.in_transaction) {
            (None, in_transaction) => *in_transaction = None,
            (Some(depth), Some(in_transaction)) => in_transaction.transaction_depth = depth,
            (Some(transaction_depth), in_transaction @ None) => {
                *in_transaction = Some(InTransactionStatus {
                    transaction_depth,
                    requires_rollback_maybe_up_to_top_level: false,
                    test_transaction: false,
                    rolled_back_by_database: false,
                });
            }
        }
    }

    /// Records that a failed statement left the connection in autocommit mode,
    /// so the database has rolled back any open transaction on its own.
    pub(crate) fn set_rolled_back_by_database(&mut self) {
        let Some(frames) = &mut self.frames else {
            self.status.set_rolled_back_by_database();
            return;
        };
        if frames.frames.is_empty() {
            // No transaction was open, so the database rolled nothing back.
            return;
        }
        frames.frames.clear();
        // Every open scope still unwinds through `rollback_transaction`.
        let Some(transaction_depth) = depth_of(frames.scopes.len()) else {
            self.status = TransactionManagerStatus::default();
            return;
        };
        let test_transaction = matches!(
            self.status,
            TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
                in_transaction: Some(InTransactionStatus {
                    test_transaction: true,
                    ..
                }),
            })
        );
        self.status = TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
            in_transaction: Some(InTransactionStatus {
                transaction_depth,
                requires_rollback_maybe_up_to_top_level: false,
                test_transaction,
                rolled_back_by_database: true,
            }),
        });
    }

    /// The frames of `conn`, if it reports its transaction control statements.
    pub(super) fn tracked_frames<Conn>(conn: &mut Conn) -> Option<&mut TransactionFrames>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        conn.transaction_state().frames.as_mut()
    }

    fn frames<Conn>(conn: &mut Conn) -> &mut TransactionFrames
    where
        Conn: Connection<TransactionManager = Self>,
    {
        Self::tracked_frames(conn).expect("Only called for connections that track their frames")
    }

    fn is_rolled_back<Conn>(conn: &mut Conn) -> QueryResult<Option<NonZeroU32>>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        Ok(Self::get_transaction_state(conn)?
            .in_transaction
            .as_ref()
            .filter(|in_transaction| in_transaction.rolled_back_by_database)
            .map(|in_transaction| in_transaction.transaction_depth))
    }

    /// Opens a transaction, or a savepoint inside an open one, as a new scope.
    ///
    /// `sql` replaces `BEGIN` and is refused inside an open transaction.
    pub(super) fn begin_tracked<Conn>(conn: &mut Conn, sql: Option<&str>) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        let state = Self::get_transaction_state(conn)?;
        if sql.is_some() && state.transaction_depth().is_some() {
            return Err(Error::AlreadyInTransaction);
        }
        let frames = Self::frames(conn);
        let scope = ScopeId(frames.next_scope);
        frames.next_scope += 1;
        let depth = frame_depth(frames.frames.len());
        let statement = match sql {
            Some(sql) => Cow::Borrowed(sql),
            None if frames.frames.is_empty() => Cow::Borrowed("BEGIN"),
            None => Cow::Owned(alloc::format!("SAVEPOINT {}", frames.generated_name())),
        };
        frames.opening = Some(scope);
        conn.instrumentation()
            .on_connection_event(InstrumentationEvent::BeginTransaction { depth });
        let result = conn.batch_execute(&statement);
        let frames = Self::frames(conn);
        let opened = frames.opening.take().is_none();
        result?;
        if !opened {
            // The statement succeeded without opening a frame, which happens
            // when an authorizer ignores it.
            return Err(Error::NotInTransaction);
        }
        frames.scopes.push(scope);
        Ok(())
    }

    pub(super) fn commit_tracked<Conn>(conn: &mut Conn) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        if Self::is_rolled_back(conn)?.is_some() {
            // Nothing of this transaction can be committed anymore.
            Self::rollback_tracked(conn)?;
            return Err(Error::TransactionRolledBackByDatabase);
        }
        let index = Self::frames(conn).close_target()?;
        Self::commit_frame(conn, index)
    }

    pub(super) fn rollback_tracked<Conn>(conn: &mut Conn) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        if let Some(depth) = Self::is_rolled_back(conn)? {
            conn.instrumentation()
                .on_connection_event(InstrumentationEvent::RollbackTransaction { depth });
            // The database has already rolled back every level of this transaction.
            Self::get_transaction_state(conn)?
                .change_transaction_depth(TransactionDepthChange::DecreaseDepth)?;
            let frames = Self::frames(conn);
            frames.scopes.pop();
            if depth.get() == 1 {
                frames.scopes.clear();
            }
            return Ok(());
        }
        let index = Self::frames(conn).close_target()?;
        Self::rollback_frame(conn, index)
    }

    /// Commits the frame at `index` together with every newer frame.
    ///
    /// A failed commit of the outermost frame that leaves the transaction open
    /// rolls it back, as [`TransactionManager::commit_transaction`] documents.
    ///
    /// [`TransactionManager::commit_transaction`]: super::TransactionManager::commit_transaction
    fn commit_frame<Conn>(conn: &mut Conn, index: usize) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        conn.instrumentation()
            .on_connection_event(InstrumentationEvent::CommitTransaction {
                depth: frame_depth(index),
            });
        let result = if index == 0 {
            conn.batch_execute("COMMIT")
        } else {
            Self::release_shadowing_frames(conn, index).and_then(|()| {
                let release =
                    alloc::format!("RELEASE SAVEPOINT {}", Self::frames(conn).sql_name(index));
                conn.batch_execute(&release)
            })
        };
        let Err(commit_error) = result else {
            return Ok(());
        };
        if index == 0
            && !Self::frames(conn).frames.is_empty()
            && let Err(rollback_error) = Self::rollback_frame(conn, 0)
        {
            conn.transaction_state().status.set_in_error();
            return Err(Error::RollbackErrorOnCommit {
                rollback_error: Box::new(rollback_error),
                commit_error: Box::new(commit_error),
            });
        }
        Err(commit_error)
    }

    /// Rolls back the frame at `index` and closes it together with every newer frame.
    fn rollback_frame<Conn>(conn: &mut Conn, index: usize) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        conn.instrumentation()
            .on_connection_event(InstrumentationEvent::RollbackTransaction {
                depth: frame_depth(index),
            });
        let result = if index == 0 {
            conn.batch_execute("ROLLBACK")
        } else {
            Self::release_shadowing_frames(conn, index).and_then(|()| {
                let frames = Self::frames(conn);
                let rollback_to =
                    alloc::format!("ROLLBACK TO SAVEPOINT {}", frames.sql_name(index));
                let release = alloc::format!("RELEASE SAVEPOINT {}", frames.sql_name(index));
                conn.batch_execute(&rollback_to)?;
                conn.batch_execute(&release)
            })
        };
        let Err(rollback_error) = result else {
            return Ok(());
        };
        let manager = conn.transaction_state();
        if manager.status.is_rolled_back_by_database() {
            // The failed statement still ended the transaction, so no
            // transaction is left to roll back.
            manager.status = TransactionManagerStatus::default();
            Self::frames(conn).scopes.clear();
        } else if index == 0 && !Self::frames(conn).frames.is_empty() {
            conn.transaction_state().status.set_in_error();
        }
        Err(rollback_error)
    }

    /// Releases the savepoints newer than the frame at `index` that share its
    /// name, so that the name refers to that frame.
    fn release_shadowing_frames<Conn>(conn: &mut Conn, index: usize) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        loop {
            let frames = Self::frames(conn);
            let Some(name) = frames.frames[index].name.as_deref() else {
                return Ok(());
            };
            if frames.position(name) == Some(index) {
                return Ok(());
            }
            let open_frames = frames.frames.len();
            let release = alloc::format!("RELEASE SAVEPOINT {}", quoted(name));
            conn.batch_execute(&release)?;
            if Self::frames(conn).frames.len() >= open_frames {
                // An authorizer ignored the release, so the name stays shadowed.
                return Ok(());
            }
        }
    }
}
