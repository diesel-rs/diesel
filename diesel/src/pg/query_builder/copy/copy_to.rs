use core::marker::PhantomData;
use std::io::BufRead;

use super::CommonOptions;
use super::CopyFormat;
use super::CopyTarget;
use crate::AppearsOnTable;
use crate::Connection;
use crate::Expression;
use crate::QueryResult;
use crate::Selectable;
use crate::deserialize::FromSqlRow;
use crate::pg::Pg;
#[cfg(feature = "postgres")]
use crate::pg::value::TypeOidLookup;
use crate::query_builder::QueryFragment;
use crate::query_builder::QueryId;
use crate::row::Row;
#[cfg(feature = "postgres")]
use crate::row::{self, Field, PartialRow, RowIndex, RowSealed};

#[derive(Default, Debug)]
pub struct CopyToOptions {
    common: CommonOptions,
    header: Option<bool>,
}

impl CopyToOptions {
    fn any_set(&self) -> bool {
        self.common.any_set() || self.header.is_some()
    }
}

impl QueryFragment<Pg> for CopyToOptions {
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()> {
        if self.any_set() {
            let mut comma = "";
            pass.push_sql(" WITH (");
            self.common.walk_ast(pass.reborrow(), &mut comma);
            if let Some(header_is_set) = self.header {
                pass.push_sql(comma);
                // commented out because rustc complains otherwise
                //comma = ", ";
                pass.push_sql("HEADER ");
                pass.push_sql(if header_is_set { "1" } else { "0" });
            }

            pass.push_sql(")");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CopyToCommand<S> {
    options: CopyToOptions,
    p: PhantomData<S>,
}

impl<S> QueryId for CopyToCommand<S>
where
    S: CopyTarget,
{
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<S> QueryFragment<Pg> for CopyToCommand<S>
where
    S: CopyTarget,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()> {
        pass.unsafe_to_cache_prepared();
        pass.push_sql("COPY ");
        S::walk_target(pass.reborrow())?;
        pass.push_sql(" TO STDOUT");
        self.options.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NotSet;

pub trait CopyToMarker: Sized {
    fn setup_options<T>(q: CopyToQuery<T, Self>) -> CopyToQuery<T, CopyToOptions>;
}

impl CopyToMarker for NotSet {
    fn setup_options<T>(q: CopyToQuery<T, Self>) -> CopyToQuery<T, CopyToOptions> {
        CopyToQuery {
            target: q.target,
            options: CopyToOptions::default(),
        }
    }
}
impl CopyToMarker for CopyToOptions {
    fn setup_options<T>(q: CopyToQuery<T, Self>) -> CopyToQuery<T, CopyToOptions> {
        q
    }
}
/// The structure returned by [`copy_to`]
///
/// The [`load`] and the [`load_raw`] methods allow
/// to receive the configured data from the database.
/// If you don't have any special needs you should prefer using
/// the more convenient `load` method.
///
/// The `with_*` methods allow to configure the settings used for the
/// copy statement.
///
/// [`load`]: CopyToQuery::load
/// [`load_raw`]: CopyToQuery::load_raw
#[derive(Debug)]
#[must_use = "`COPY TO` statements are only executed when calling `.load()` or `load_raw()`."]
#[cfg(feature = "postgres_backend")]
pub struct CopyToQuery<T, O> {
    target: T,
    options: O,
}

#[cfg(feature = "postgres")]
struct CopyRow<'a> {
    buffers: Vec<Option<&'a [u8]>>,
    result: &'a crate::pg::connection::PgResult,
}

#[cfg(feature = "postgres")]
struct CopyField<'a> {
    field: &'a Option<&'a [u8]>,
    result: &'a crate::pg::connection::PgResult,
    col_idx: usize,
}

#[cfg(feature = "postgres")]
impl<'f> Field<'f, Pg> for CopyField<'f> {
    fn field_name(&self) -> Option<&str> {
        None
    }

    fn value(&self) -> Option<<Pg as crate::backend::Backend>::RawValue<'_>> {
        let value = self.field.as_deref()?;
        Some(crate::pg::PgValue::new_internal(value, self))
    }
}

#[cfg(feature = "postgres")]
impl TypeOidLookup for CopyField<'_> {
    fn lookup(&self) -> core::num::NonZeroU32 {
        self.result.column_type(self.col_idx)
    }
}

#[cfg(feature = "postgres")]
impl RowSealed for CopyRow<'_> {}

#[cfg(feature = "postgres")]
impl RowIndex<usize> for CopyRow<'_> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

#[cfg(feature = "postgres")]
impl<'a> RowIndex<&'a str> for CopyRow<'_> {
    fn idx(&self, _idx: &'a str) -> Option<usize> {
        None
    }
}

#[cfg(feature = "postgres")]
impl<'a> Row<'a, Pg> for CopyRow<'_> {
    type Field<'f>
        = CopyField<'f>
    where
        'a: 'f,
        Self: 'f;

    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.buffers.len()
    }

    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'a: 'b,
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        let buffer = self.buffers.get(idx)?;
        Some(CopyField {
            field: buffer,
            result: self.result,
            col_idx: idx,
        })
    }

    fn partial_row(
        &self,
        range: core::ops::Range<usize>,
    ) -> row::PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

pub trait ExecuteCopyToConnection: Connection<Backend = Pg> {
    type CopyToBuffer<'a>: BufRead;

    fn make_row<'a, 'b>(
        out: &'a Self::CopyToBuffer<'_>,
        buffers: Vec<Option<&'a [u8]>>,
    ) -> impl Row<'b, Pg> + 'a;

    fn get_buffer<'a>(out: &'a Self::CopyToBuffer<'_>) -> &'a [u8];

    fn execute<T>(&mut self, command: CopyToCommand<T>) -> QueryResult<Self::CopyToBuffer<'_>>
    where
        T: CopyTarget;
}

#[cfg(feature = "postgres")]
impl ExecuteCopyToConnection for crate::PgConnection {
    type CopyToBuffer<'a> = crate::pg::connection::copy::CopyToBuffer<'a>;

    fn make_row<'a, 'b>(
        out: &'a Self::CopyToBuffer<'_>,
        buffers: Vec<Option<&'a [u8]>>,
    ) -> impl Row<'b, Pg> + 'a {
        CopyRow {
            buffers,
            result: out.get_result(),
        }
    }

    fn get_buffer<'a>(out: &'a Self::CopyToBuffer<'_>) -> &'a [u8] {
        out.data_slice()
    }

    fn execute<T>(&mut self, command: CopyToCommand<T>) -> QueryResult<Self::CopyToBuffer<'_>>
    where
        T: CopyTarget,
    {
        self.copy_to(command)
    }
}

#[cfg(feature = "r2d2")]
impl<C> ExecuteCopyToConnection for crate::r2d2::PooledConnection<crate::r2d2::ConnectionManager<C>>
where
    C: ExecuteCopyToConnection + crate::r2d2::R2D2Connection + 'static,
{
    type CopyToBuffer<'a> = C::CopyToBuffer<'a>;

    fn make_row<'a, 'b>(
        out: &'a Self::CopyToBuffer<'_>,
        buffers: Vec<Option<&'a [u8]>>,
    ) -> impl Row<'b, Pg> + 'a {
        C::make_row(out, buffers)
    }

    fn get_buffer<'a>(out: &'a Self::CopyToBuffer<'_>) -> &'a [u8] {
        C::get_buffer(out)
    }

    fn execute<T>(&mut self, command: CopyToCommand<T>) -> QueryResult<Self::CopyToBuffer<'_>>
    where
        T: CopyTarget,
    {
        C::execute(&mut **self, command)
    }
}

impl<T> CopyToQuery<T, NotSet>
where
    T: CopyTarget,
{
    /// Copy data from the database by returning an iterator of deserialized data
    ///
    /// This function allows to easily load data from the database via a `COPY TO` statement.
    /// It does **not** allow to configure any settings via the `with_*` method, as it internally
    /// sets the required options itself. It will use the binary format to deserialize the result
    /// into the specified type `U`. Column selection is performed via [`Selectable`].
    pub fn load<U, C>(self, conn: &mut C) -> QueryResult<impl Iterator<Item = QueryResult<U>> + '_>
    where
        U: FromSqlRow<<U::SelectExpression as Expression>::SqlType, Pg> + Selectable<Pg>,
        U::SelectExpression: AppearsOnTable<T::Table> + CopyTarget<Table = T::Table>,
        C: ExecuteCopyToConnection,
    {
        let command = CopyToCommand {
            p: PhantomData::<U::SelectExpression>,
            options: CopyToOptions {
                header: None,
                common: CommonOptions {
                    format: Some(CopyFormat::Binary),
                    ..Default::default()
                },
            },
        };
        let out = ExecuteCopyToConnection::execute(conn, command)?;
        decode_data::<U, C>(out)
    }
}

fn decode_data<'a, U, C>(
    mut out: C::CopyToBuffer<'a>,
) -> QueryResult<impl Iterator<Item = QueryResult<U>> + 'a>
where
    U: FromSqlRow<<U::SelectExpression as Expression>::SqlType, Pg> + Selectable<Pg>,
    C: ExecuteCopyToConnection + 'a,
{
    let io_result_mapper = |e| crate::result::Error::DeserializationError(Box::new(e));
    // see https://www.postgresql.org/docs/current/sql-copy.html for
    // a description of the binary format
    //
    // We don't write oids

    let buffer = out.fill_buf().map_err(io_result_mapper)?;
    expect_buffer_len(buffer, super::COPY_MAGIC_HEADER.len() + 8)?;
    if buffer[..super::COPY_MAGIC_HEADER.len()] != super::COPY_MAGIC_HEADER {
        return Err(crate::result::Error::DeserializationError(
            "Unexpected protocol header".into(),
        ));
    }
    // we care only about bit 16-31 here, so we can just skip the bytes in between
    let flags_backward_incompatible = i16::from_be_bytes(
        (&buffer[super::COPY_MAGIC_HEADER.len() + 2..super::COPY_MAGIC_HEADER.len() + 4])
            .try_into()
            .expect("Exactly 2 byte"),
    );
    if flags_backward_incompatible != 0 {
        return Err(crate::result::Error::DeserializationError(
            format!("Unexpected flag value: {flags_backward_incompatible:x}").into(),
        ));
    }
    let header_size = usize::try_from(i32::from_be_bytes(
        (&buffer[super::COPY_MAGIC_HEADER.len() + 4..super::COPY_MAGIC_HEADER.len() + 8])
            .try_into()
            .expect("Exactly 4 byte"),
    ))
    .map_err(|e| crate::result::Error::DeserializationError(Box::new(e)))?;
    out.consume(super::COPY_MAGIC_HEADER.len() + 8 + header_size);
    let mut len = None;
    let mut finished = false;
    Ok(core::iter::from_fn(move || {
        if finished {
            return None;
        }
        if let Some(len) = len {
            out.consume(len);
            if let Err(e) = out.fill_buf().map_err(io_result_mapper) {
                finished = true;
                return Some(Err(e));
            }
        }
        let buffer = C::get_buffer(&out);
        len = Some(buffer.len());
        if let Err(e) = expect_buffer_len(buffer, 2) {
            finished = true;
            return Some(Err(e));
        }
        let tuple_count = i16::from_be_bytes((&buffer[..2]).try_into().expect("Exactly 2 bytes"));
        if tuple_count > 0 {
            let tuple_count = match usize::try_from(tuple_count) {
                Ok(o) => o,
                Err(e) => {
                    return Some(Err(crate::result::Error::DeserializationError(Box::new(e))));
                }
            };
            let mut buffers = Vec::with_capacity(tuple_count);
            let mut offset = 2;
            for _t in 0..tuple_count {
                if let Err(e) = expect_buffer_len(buffer, offset + 4) {
                    finished = true;
                    return Some(Err(e));
                }
                let data_size = i32::from_be_bytes(
                    (&buffer[offset..offset + 4])
                        .try_into()
                        .expect("Exactly 4 bytes"),
                );
                if data_size < 0 {
                    offset += 4;
                    buffers.push(None);
                } else {
                    match usize::try_from(data_size) {
                        Ok(data_size) => {
                            if let Err(e) = expect_buffer_len(buffer, offset + 4 + data_size) {
                                finished = true;
                                return Some(Err(e));
                            }
                            buffers.push(Some(&buffer[offset + 4..offset + 4 + data_size]));
                            offset += data_size + 4;
                        }
                        Err(e) => {
                            return Some(Err(crate::result::Error::DeserializationError(
                                Box::new(e),
                            )));
                        }
                    }
                }
            }

            let row = C::make_row(&out, buffers);
            Some(U::build_from_row(&row).map_err(crate::result::Error::DeserializationError))
        } else {
            out.consume(len.expect("It's always set here"));
            finished = true;
            if let Err(e) = out.fill_buf().map_err(io_result_mapper) {
                return Some(Err(e));
            }
            None
        }
    }))
}

fn expect_buffer_len(buffer: &[u8], expected: usize) -> QueryResult<()> {
    if buffer.len() < expected {
        Err(crate::result::Error::DeserializationError(
            format!(
                "Expected at least {expected} bytes but got {} bytes",
                buffer.len()
            )
            .into(),
        ))
    } else {
        Ok(())
    }
}

impl<T, O> CopyToQuery<T, O>
where
    O: CopyToMarker,
    T: CopyTarget,
{
    /// Copy data from the database by directly accessing the provided response
    ///
    /// This function returns a type that implements [`std::io::BufRead`] which allows to directly read
    /// the data as provided by the database. The exact format depends on what options are
    /// set via the various `with_*` methods.
    pub fn load_raw<C>(self, conn: &mut C) -> QueryResult<impl BufRead + '_>
    where
        C: ExecuteCopyToConnection,
    {
        let q = O::setup_options(self);
        let command = CopyToCommand {
            p: PhantomData::<T>,
            options: q.options,
        };
        ExecuteCopyToConnection::execute(conn, command)
    }

    /// The format used for the copy statement
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_format(self, format: CopyFormat) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.format = Some(format);
        out
    }

    /// Whether or not the `freeze` option is set
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_freeze(self, freeze: bool) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.freeze = Some(freeze);
        out
    }

    /// Which delimiter should be used for textual output formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    ///
    /// Diesel will automatically escape the provided delimiter for the default
    /// PostgreSQL setting `standard_conforming_strings=on`. If you use a non-standard
    /// conforming setting you need to take care of escaping the value on your own
    pub fn with_delimiter(self, delimiter: char) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.delimiter = Some(delimiter);
        out
    }

    /// Which string should be used in place of a `NULL` value
    /// for textual output formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    ///
    /// Diesel will automatically escape the provided delimiter for the default
    /// PostgreSQL setting `standard_conforming_strings=on`. If you use a non-standard
    /// conforming setting you need to take care of escaping the value on your own
    pub fn with_null(self, null: impl Into<String>) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.null = Some(null.into());
        out
    }

    /// Which quote character should be used for textual output formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    ///
    /// Diesel will automatically escape the provided delimiter for the default
    /// PostgreSQL setting `standard_conforming_strings=on`. If you use a non-standard
    /// conforming setting you need to take care of escaping the value on your own
    pub fn with_quote(self, quote: char) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.quote = Some(quote);
        out
    }

    /// Which escape character should be used for textual output formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    ///
    /// Diesel will automatically escape the provided delimiter for the default
    /// PostgreSQL setting `standard_conforming_strings=on`. If you use a non-standard
    /// conforming setting you need to take care of escaping the value on your own
    pub fn with_escape(self, escape: char) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.escape = Some(escape);
        out
    }

    /// Is a header provided as part of the textual input or not
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_header(self, set: bool) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.header = Some(set);
        out
    }
}

/// Creates a `COPY TO` statement
///
/// This function constructs a `COPY TO` statement which copies data
/// from the database **to** a client side target. It's designed to move
/// larger amounts of data out of the database.
///
/// This function accepts a target selection (table name or list of columns) as argument.
///
/// There are two ways to use a `COPY TO` statement with diesel:
///
/// * By using [`CopyToQuery::load`] directly to load the deserialized result
///   directly into a specified type
/// * By using the `with_*` methods to configure the format sent by the database
///   and then by calling [`CopyToQuery::load_raw`] to receive the raw data
///   sent by the database.
///
/// The first variant uses the `BINARY` format internally to receive
/// the selected data efficiently. It automatically sets the right options
/// and does not allow to change them via `with_*` methods.
///
/// The second variant allows you to control the behaviour of the
/// generated `COPY TO` statement in detail. You can use the various
/// `with_*` methods for that before issuing the statement via [`CopyToQuery::load_raw`].
/// That method will return an type that implements [`std::io::BufRead`], which
/// allows you to directly read the response from the database in the configured
/// format.
/// See [the postgresql documentation](https://www.postgresql.org/docs/current/sql-copy.html)
/// for more details about the supported formats.
///
/// If you don't have any specific needs you should prefer using the more
/// convenient first variant.
///
/// This functionality is postgresql specific.
///
/// # Examples
///
/// ## Via [`CopyToQuery::load()`]
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # use crate::schema::users;
///
/// #[derive(Queryable, Selectable, PartialEq, Debug)]
/// #[diesel(table_name = users)]
/// #[diesel(check_for_backend(diesel::pg::Pg))]
/// struct User {
///     name: String,
/// }
///
/// # fn run_test() -> QueryResult<()> {
/// # let connection = &mut establish_connection();
/// let out = diesel::copy_to(users::table)
///     .load::<User, _>(connection)?
///     .collect::<Result<Vec<_>, _>>()?;
///
/// assert_eq!(
///     out,
///     vec![
///         User {
///             name: "Sean".into()
///         },
///         User {
///             name: "Tess".into()
///         }
///     ]
/// );
/// # Ok(())
/// # }
/// # fn main() {
/// #    run_test().unwrap();
/// # }
/// ```
///
/// ## Via [`CopyToQuery::load_raw()`]
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # fn run_test() -> QueryResult<()> {
/// # use crate::schema::users;
/// use diesel::pg::CopyFormat;
/// use std::io::Read;
/// # let connection = &mut establish_connection();
///
/// let mut copy = diesel::copy_to(users::table)
///     .with_format(CopyFormat::Csv)
///     .load_raw(connection)?;
///
/// let mut out = String::new();
/// copy.read_to_string(&mut out).unwrap();
/// assert_eq!(out, "1,Sean\n2,Tess\n");
/// # Ok(())
/// # }
/// # fn main() {
/// #    run_test().unwrap();
/// # }
/// ```
#[cfg(feature = "postgres_backend")]
pub fn copy_to<T>(target: T) -> CopyToQuery<T, NotSet>
where
    T: CopyTarget,
{
    CopyToQuery {
        target,
        options: NotSet,
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::connection::Instrumentation;
    use crate::connection::{AnsiTransactionManager, TransactionManager};
    use crate::pg::query_builder::copy::COPY_MAGIC_HEADER;
    use std::collections::VecDeque;
    use std::io::Read;

    struct Wrapper;

    /// Mimics `CopyToBuffer`: hands out one `CopyData` message at a time
    /// and only moves on to the next one once the current one is consumed
    struct Messages {
        messages: VecDeque<Vec<u8>>,
        offset: usize,
    }

    impl Messages {
        fn new(messages: Vec<Vec<u8>>) -> Self {
            Self {
                messages: messages.into(),
                offset: 0,
            }
        }

        fn data_slice(&self) -> &[u8] {
            self.messages
                .front()
                .map(|m| &m[self.offset..])
                .unwrap_or_default()
        }
    }

    impl Read for Messages {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let data = self.fill_buf()?;
            let len = usize::min(buf.len(), data.len());
            buf[..len].copy_from_slice(&data[..len]);
            self.consume(len);
            Ok(len)
        }
    }

    impl BufRead for Messages {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            if self.data_slice().is_empty() && self.messages.pop_front().is_some() {
                self.offset = 0;
            }
            Ok(self.data_slice())
        }

        fn consume(&mut self, amt: usize) {
            let len = self.messages.front().map_or(0, Vec::len);
            self.offset = usize::min(len, self.offset + amt);
        }
    }

    impl crate::connection::SimpleConnection for Wrapper {
        fn batch_execute(&mut self, _query: &str) -> QueryResult<()> {
            unreachable!()
        }
    }

    impl crate::connection::ConnectionSealed for Wrapper {}

    impl crate::connection::Connection for Wrapper {
        type Backend = crate::pg::Pg;

        type TransactionManager = crate::connection::AnsiTransactionManager;

        fn establish(_database_url: &str) -> crate::prelude::ConnectionResult<Self> {
            unreachable!()
        }

        fn execute_returning_count<T>(&mut self, _source: &T) -> QueryResult<usize>
        where
            T: QueryFragment<Self::Backend> + QueryId,
        {
            unreachable!()
        }

        fn transaction_state(
            &mut self,
        ) -> &mut <AnsiTransactionManager as TransactionManager<Self>>::TransactionStateData
        {
            unreachable!()
        }

        fn instrumentation(&mut self) -> &mut dyn Instrumentation {
            unreachable!()
        }

        fn set_instrumentation(
            &mut self,
            _instrumentation: impl crate::connection::Instrumentation,
        ) {
            unreachable!()
        }

        fn set_prepared_statement_cache_size(&mut self, _size: crate::connection::CacheSize) {
            unreachable!()
        }
    }

    impl ExecuteCopyToConnection for Wrapper {
        type CopyToBuffer<'a> = Messages;

        fn make_row<'a, 'b>(
            _out: &'a Self::CopyToBuffer<'_>,
            _buffers: Vec<Option<&'a [u8]>>,
        ) -> impl Row<'b, Pg> + 'a {
            // the tests below never decode a complete row
            unreachable!() as CopyRow<'a>
        }

        fn get_buffer<'a>(out: &'a Self::CopyToBuffer<'_>) -> &'a [u8] {
            out.data_slice()
        }

        fn execute<T>(&mut self, _command: CopyToCommand<T>) -> QueryResult<Self::CopyToBuffer<'_>>
        where
            T: CopyTarget,
        {
            unreachable!()
        }
    }

    crate::table! {
        users {
            id -> Integer,
            name -> Text,
            hair_color -> Nullable<Text>,
            age -> Nullable<Integer>,
        }
    }

    #[derive(diesel::Queryable, diesel::Selectable, Debug)]
    #[diesel(table_name = users)]
    #[allow(dead_code, reason = "Only used as structure for the test")]
    struct Dummy {
        id: i32,
        name: String,
        hair_color: Option<String>,
        age: Option<i32>,
    }

    fn decode(messages: Vec<Vec<u8>>) -> QueryResult<impl Iterator<Item = QueryResult<Dummy>>> {
        decode_data::<Dummy, Wrapper>(Messages::new(messages))
    }

    fn tuple_count(n: i16) -> [u8; 2] {
        n.to_be_bytes()
    }

    fn field_len(n: i32) -> [u8; 4] {
        n.to_be_bytes()
    }

    #[track_caller]
    fn first_row_error(first_message: &[&[u8]]) -> String {
        let first_message = [
            COPY_MAGIC_HEADER.as_slice(),
            [0; 8].as_slice(),
            &first_message.concat(),
        ]
        .concat();
        let mut rows = decode(vec![first_message]).unwrap();
        let error = match rows.next() {
            Some(Err(e)) => e.to_string(),
            Some(Ok(row)) => panic!("expected an error, got {row:?}"),
            None => panic!("expected an error, got the end of the data"),
        };
        assert!(
            rows.next().is_none(),
            "the iterator must end after an error"
        );
        error
    }

    #[track_caller]
    fn check_decode_error(input: Vec<Vec<u8>>, error: &str) {
        let r = decode(input);
        assert!(r.is_err());
        let Err(e) = r else { unreachable!() };
        assert_eq!(e.to_string(), error);
    }

    #[test]
    fn check_empty_buffer_returns_error() {
        check_decode_error(
            vec![Vec::new()],
            "Expected at least 19 bytes but got 0 bytes",
        );
    }

    #[test]
    fn check_header_without_flags_returns_error() {
        check_decode_error(
            vec![COPY_MAGIC_HEADER.to_vec()],
            "Expected at least 19 bytes but got 11 bytes",
        );
    }

    #[test]
    fn check_header_without_extension_length_returns_error() {
        check_decode_error(
            vec![[COPY_MAGIC_HEADER.as_slice(), &[0; 7]].concat().to_vec()],
            "Expected at least 19 bytes but got 18 bytes",
        );
    }

    #[test]
    fn check_missing_trailer_returns_error() {
        // the data ends right after the header
        assert_eq!(
            first_row_error(&[]),
            "Expected at least 2 bytes but got 0 bytes",
        );
    }

    #[test]
    fn check_truncated_tuple_count_returns_error() {
        assert_eq!(
            first_row_error(&[&[0]]),
            "Expected at least 2 bytes but got 1 bytes"
        );
    }

    #[test]
    fn check_missing_field_length_returns_error() {
        assert_eq!(
            first_row_error(&[&tuple_count(4)]),
            "Expected at least 6 bytes but got 2 bytes"
        );
    }

    #[test]
    fn check_truncated_field_length_returns_error() {
        assert_eq!(
            first_row_error(&[&tuple_count(4), &[0, 0]]),
            "Expected at least 6 bytes but got 4 bytes"
        );
    }

    #[test]
    fn check_missing_field_length_after_value_returns_error() {
        assert_eq!(
            first_row_error(&[&tuple_count(4), &field_len(4), &1_i32.to_be_bytes()]),
            "Expected at least 14 bytes but got 10 bytes"
        );
    }

    #[test]
    fn check_missing_field_length_after_null_returns_error() {
        assert_eq!(
            first_row_error(&[&tuple_count(4), &field_len(-1)]),
            "Expected at least 10 bytes but got 6 bytes"
        );
    }

    #[test]
    fn check_missing_field_data_returns_error() {
        assert_eq!(
            first_row_error(&[&tuple_count(4), &field_len(4)]),
            "Expected at least 10 bytes but got 6 bytes"
        );
    }

    #[test]
    fn check_truncated_field_data_returns_error() {
        assert_eq!(
            first_row_error(&[&tuple_count(4), &field_len(100), b"abc"]),
            "Expected at least 106 bytes but got 9 bytes"
        );
    }

    #[test]
    fn check_truncated_later_field_data_returns_error() {
        assert_eq!(
            first_row_error(&[
                &tuple_count(4),
                &field_len(4),
                &1_i32.to_be_bytes(),
                &field_len(5),
                b"Sean",
            ]),
            "Expected at least 19 bytes but got 18 bytes"
        );
    }

    #[test]
    fn check_field_length_i32_max_returns_error() {
        assert_eq!(
            first_row_error(&[&tuple_count(4), &field_len(i32::MAX), b"abc"]),
            format!(
                "Expected at least {} bytes but got 9 bytes",
                6 + i32::MAX as usize
            )
        );
    }
}
