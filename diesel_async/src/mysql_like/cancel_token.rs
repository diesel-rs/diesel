use core::marker::PhantomData;

use diesel::mysql_like::MysqlLikeBackend;
use mysql_async::prelude::Query;
use mysql_async::{Opts, OptsBuilder};

use crate::mysql_like::error_helper::ErrorHelper;

/// The capability to request cancellation of in-progress queries on a
/// connection.
#[derive(Clone)]
pub struct MysqlLikeCancelToken<DB: MysqlLikeBackend> {
    pub(crate) opts: Opts,
    pub(crate) kill_id: u32,
    pub(crate) phantom: PhantomData<DB>,
}

#[expect(private_bounds)]
impl<DB: MysqlLikeBackend> MysqlLikeCancelToken<DB>
where
    diesel::result::Error: From<ErrorHelper<DB>>,
{
    /// Attempts to cancel the in-progress query on the connection associated
    /// with this `CancelToken`.
    ///
    /// The server provides no information about whether a cancellation attempt was successful or not. An error will
    /// only be returned if the client was unable to connect to the database.
    ///
    /// Cancellation is inherently racy. There is no guarantee that the
    /// cancellation request will reach the server before the query terminates
    /// normally, or that the connection associated with this token is still
    /// active.
    pub async fn cancel_query(&self) -> diesel::result::ConnectionResult<()> {
        let builder = OptsBuilder::from_opts(self.opts.clone());

        let conn = mysql_async::Conn::new(builder)
            .await
            .map_err(ErrorHelper::<DB>::new)?;

        format!("KILL QUERY {};", self.kill_id)
            .ignore(conn)
            .await
            .map_err(ErrorHelper::<DB>::new)?;

        Ok(())
    }
}
