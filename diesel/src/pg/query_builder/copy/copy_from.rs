use std::borrow::Cow;
use std::marker::PhantomData;

use byteorder::NetworkEndian;
use byteorder::WriteBytesExt;

use super::CommonOptions;
use super::CopyFormat;
use super::CopyTarget;
use crate::expression::bound::Bound;
use crate::insertable::ColumnInsertValue;
use crate::pg::backend::FailedToLookupTypeError;
use crate::pg::metadata_lookup::PgMetadataCacheKey;
use crate::pg::Pg;
use crate::pg::PgMetadataLookup;
use crate::query_builder::BatchInsert;
use crate::query_builder::QueryFragment;
use crate::query_builder::QueryId;
use crate::query_builder::ValuesClause;
use crate::serialize::IsNull;
use crate::serialize::ToSql;
use crate::Connection;
use crate::Insertable;
use crate::QueryResult;
use crate::{Column, Table};

/// Describes the different possible settings for the `HEADER` option
/// for `COPY FROM` statements
#[derive(Debug, Copy, Clone)]
pub enum CopyHeader {
    /// Is the header set?
    Set(bool),
    /// Match the header with the targeted table names
    /// and fail in the case of a mismatch
    Match,
}

#[derive(Debug, Default)]
pub struct CopyFromOptions {
    common: CommonOptions,
    default: Option<String>,
    header: Option<CopyHeader>,
}

impl QueryFragment<Pg> for CopyFromOptions {
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()> {
        if self.any_set() {
            let mut comma = "";
            pass.push_sql(" WITH (");
            self.common.walk_ast(pass.reborrow(), &mut comma);
            if let Some(ref default) = self.default {
                pass.push_sql(comma);
                comma = ", ";
                pass.push_sql("DEFAULT '");
                // cannot use binds here :(
                pass.push_sql(default);
                pass.push_sql("'");
            }
            if let Some(ref header) = self.header {
                pass.push_sql(comma);
                // commented out because rustc complains otherwise
                //comma = ", ";
                pass.push_sql("HEADER ");
                match header {
                    CopyHeader::Set(true) => pass.push_sql("1"),
                    CopyHeader::Set(false) => pass.push_sql("0"),
                    CopyHeader::Match => pass.push_sql("MATCH"),
                }
            }

            pass.push_sql(")");
        }
        Ok(())
    }
}

impl CopyFromOptions {
    fn any_set(&self) -> bool {
        self.common.any_set() || self.default.is_some() || self.header.is_some()
    }
}

#[derive(Debug)]
pub struct CopyFrom<S, F> {
    options: CopyFromOptions,
    copy_callback: F,
    p: PhantomData<S>,
}

pub(crate) struct InternalCopyFromQuery<S, T> {
    pub(crate) target: S,
    p: PhantomData<T>,
}

#[cfg(feature = "postgres")]
impl<S, T> InternalCopyFromQuery<S, T> {
    pub(crate) fn new(target: S) -> Self {
        Self {
            target,
            p: PhantomData,
        }
    }
}

impl<S, T> QueryId for InternalCopyFromQuery<S, T>
where
    S: CopyFromExpression<T>,
{
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<S, T> QueryFragment<Pg> for InternalCopyFromQuery<S, T>
where
    S: CopyFromExpression<T>,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()> {
        pass.unsafe_to_cache_prepared();
        pass.push_sql("COPY ");
        self.target.walk_target(pass.reborrow())?;
        pass.push_sql(" FROM STDIN");
        self.target.options().walk_ast(pass.reborrow())?;
        Ok(())
    }
}

pub trait CopyFromExpression<T> {
    type Error: From<crate::result::Error> + std::error::Error;

    fn callback(&mut self, copy: &mut impl std::io::Write) -> Result<(), Self::Error>;

    fn walk_target<'b>(
        &'b self,
        pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()>;

    fn options(&self) -> &CopyFromOptions;
}

impl<S, F, E> CopyFromExpression<S::Table> for CopyFrom<S, F>
where
    E: From<crate::result::Error> + std::error::Error,
    S: CopyTarget,
    F: Fn(&mut dyn std::io::Write) -> Result<(), E>,
{
    type Error = E;

    fn callback(&mut self, copy: &mut impl std::io::Write) -> Result<(), Self::Error> {
        (self.copy_callback)(copy)
    }

    fn options(&self) -> &CopyFromOptions {
        &self.options
    }

    fn walk_target<'b>(
        &'b self,
        pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()> {
        S::walk_target(pass)
    }
}

struct Dummy;

impl PgMetadataLookup for Dummy {
    fn lookup_type(&mut self, type_name: &str, schema: Option<&str>) -> crate::pg::PgTypeMetadata {
        let cache_key = PgMetadataCacheKey::new(
            schema.map(Into::into).map(Cow::Owned),
            Cow::Owned(type_name.into()),
        );
        crate::pg::PgTypeMetadata(Err(FailedToLookupTypeError::new_internal(cache_key)))
    }
}

trait CopyFromInsertableHelper {
    type Target: CopyTarget;
    const COLUMN_COUNT: i16;

    fn write_to_buffer(&self, idx: i16, out: &mut Vec<u8>) -> QueryResult<IsNull>;
}

macro_rules! impl_copy_from_insertable_helper_for_values_clause {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<T, $($ST,)* $($T,)* $($TT,)*> CopyFromInsertableHelper for ValuesClause<
                ($(ColumnInsertValue<$ST, Bound<$T, $TT>>,)*),
            T>
                where
                T: Table,
                $($ST: Column<Table = T>,)*
                ($($ST,)*): CopyTarget,
                $($TT: ToSql<$T, Pg>,)*
            {
                type Target = ($($ST,)*);
                const COLUMN_COUNT: i16 = $Tuple as i16;

                fn write_to_buffer(&self, idx: i16, out: &mut Vec<u8>) -> QueryResult<IsNull> {
                    use crate::query_builder::ByteWrapper;
                    use crate::serialize::Output;

                    let values = &self.values;
                    match idx {
                        $($idx =>{
                            let item = &values.$idx.expr.item;
                            let is_null = ToSql::<$T, Pg>::to_sql(
                                item,
                                &mut Output::new( ByteWrapper(out), &mut Dummy as _)
                            ).map_err(crate::result::Error::SerializationError)?;
                            return Ok(is_null);
                        })*
                        _ => unreachable!(),
                    }
                }
            }

            impl<'a, T, $($ST,)* $($T,)* $($TT,)*> CopyFromInsertableHelper for ValuesClause<
                ($(ColumnInsertValue<$ST, &'a Bound<$T, $TT>>,)*),
            T>
                where
                T: Table,
                $($ST: Column<Table = T>,)*
                ($($ST,)*): CopyTarget,
                $($TT: ToSql<$T, Pg>,)*
            {
                type Target = ($($ST,)*);
                const COLUMN_COUNT: i16 = $Tuple as i16;

                fn write_to_buffer(&self, idx: i16, out: &mut Vec<u8>) -> QueryResult<IsNull> {
                    use crate::query_builder::ByteWrapper;
                    use crate::serialize::Output;

                    let values = &self.values;
                    match idx {
                        $($idx =>{
                            let item = &values.$idx.expr.item;
                            let is_null = ToSql::<$T, Pg>::to_sql(
                                item,
                                &mut Output::new( ByteWrapper(out), &mut Dummy as _)
                            ).map_err(crate::result::Error::SerializationError)?;
                            return Ok(is_null);
                        })*
                        _ => unreachable!(),
                    }
                }
            }
        )*
    }
}

diesel_derives::__diesel_for_each_tuple!(impl_copy_from_insertable_helper_for_values_clause);

#[derive(Debug)]
pub struct InsertableWrapper<I>(Option<I>);

impl<I, T, V, QId, const STATIC_QUERY_ID: bool> CopyFromExpression<T> for InsertableWrapper<I>
where
    I: Insertable<T, Values = BatchInsert<Vec<V>, T, QId, STATIC_QUERY_ID>>,
    V: CopyFromInsertableHelper,
{
    type Error = crate::result::Error;

    fn callback(&mut self, copy: &mut impl std::io::Write) -> Result<(), Self::Error> {
        let io_result_mapper = |e| crate::result::Error::DeserializationError(Box::new(e));
        // see https://www.postgresql.org/docs/current/sql-copy.html for
        // a description of the binary format
        //
        // We don't write oids

        // write the header
        copy.write_all(&super::COPY_MAGIC_HEADER)
            .map_err(io_result_mapper)?;
        copy.write_i32::<NetworkEndian>(0)
            .map_err(io_result_mapper)?;
        copy.write_i32::<NetworkEndian>(0)
            .map_err(io_result_mapper)?;
        // write the data
        // we reuse the same buffer here again and again
        // as we expect the data to be "similar"
        // this skips reallocating
        let mut buffer = Vec::<u8>::new();
        let values = self
            .0
            .take()
            .expect("We only call this callback once")
            .values();
        for i in values.values {
            // column count
            buffer
                .write_i16::<NetworkEndian>(V::COLUMN_COUNT)
                .map_err(io_result_mapper)?;
            for idx in 0..V::COLUMN_COUNT {
                // first write the null indicator as dummy value
                buffer
                    .write_i32::<NetworkEndian>(-1)
                    .map_err(io_result_mapper)?;
                let len_before = buffer.len();
                let is_null = i.write_to_buffer(idx, &mut buffer)?;
                if is_null == IsNull::No {
                    // fill in the length afterwards
                    let len_after = buffer.len();
                    let diff = (len_after - len_before) as i32;
                    let bytes = i32::to_be_bytes(diff);
                    for (b, t) in bytes.into_iter().zip(&mut buffer[len_before - 4..]) {
                        *t = b;
                    }
                }
            }
            copy.write_all(&buffer).map_err(io_result_mapper)?;
            buffer.clear();
        }
        // write the trailer
        copy.write_i16::<NetworkEndian>(-1)
            .map_err(io_result_mapper)?;
        Ok(())
    }

    fn options(&self) -> &CopyFromOptions {
        &CopyFromOptions {
            common: CommonOptions {
                format: Some(CopyFormat::Binary),
                freeze: None,
                delimiter: None,
                null: None,
                quote: None,
                escape: None,
            },
            default: None,
            header: None,
        }
    }

    fn walk_target<'b>(
        &'b self,
        pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()> {
        <V as CopyFromInsertableHelper>::Target::walk_target(pass)
    }
}

/// The structure returned by [`copy_from`]
///
/// The [`from_raw_data`] and the [`from_insertable`] methods allow
/// to configure the data copied into the database
///
/// The `with_*` methods allow to configure the settings used for the
/// copy statement.
///
/// [`from_raw_data`]: CopyFromQuery::from_raw_data
/// [`from_insertable`]: CopyFromQuery::from_insertable
#[derive(Debug)]
#[must_use = "`COPY FROM` statements are only executed when calling `.execute()`."]
#[cfg(feature = "postgres_backend")]
pub struct CopyFromQuery<T, Action> {
    table: T,
    action: Action,
}

impl<T> CopyFromQuery<T, NotSet>
where
    T: Table,
{
    /// Copy data into the database by directly providing the data in the corresponding format
    ///
    /// `target` specifies the column selection that is the target of the `COPY FROM` statement
    /// `action` expects a callback which accepts a [`std::io::Write`] argument. The necessary format
    /// accepted by this writer sink depends on the options provided via the `with_*` methods
    #[allow(clippy::wrong_self_convention)] // the sql struct is named that way
    pub fn from_raw_data<F, C, E>(self, _target: C, action: F) -> CopyFromQuery<T, CopyFrom<C, F>>
    where
        C: CopyTarget<Table = T>,
        F: Fn(&mut dyn std::io::Write) -> Result<(), E>,
    {
        CopyFromQuery {
            table: self.table,
            action: CopyFrom {
                p: PhantomData,
                options: Default::default(),
                copy_callback: action,
            },
        }
    }

    /// Copy a set of insertable values into the database.
    ///
    /// The `insertable` argument is expected to be a `Vec<I>`, `&[I]` or similar, where `I`
    /// needs to implement `Insertable<T>`. If you use the [`#[derive(Insertable)]`](derive@crate::prelude::Insertable)
    /// derive macro make sure to also set the `#[diesel(treat_none_as_default_value = false)]` option
    /// to disable the default value handling otherwise implemented by `#[derive(Insertable)]`.
    ///
    /// This uses the binary format. It internally configures the correct
    /// set of settings and does not allow to set other options
    #[allow(clippy::wrong_self_convention)] // the sql struct is named that way
    pub fn from_insertable<I>(self, insertable: I) -> CopyFromQuery<T, InsertableWrapper<I>>
    where
        InsertableWrapper<I>: CopyFromExpression<T>,
    {
        CopyFromQuery {
            table: self.table,
            action: InsertableWrapper(Some(insertable)),
        }
    }
}

impl<T, C, F> CopyFromQuery<T, CopyFrom<C, F>> {
    /// The format used for the copy statement
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_format(mut self, format: CopyFormat) -> Self {
        self.action.options.common.format = Some(format);
        self
    }

    /// Whether or not the `freeze` option is set
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_freeze(mut self, freeze: bool) -> Self {
        self.action.options.common.freeze = Some(freeze);
        self
    }

    /// Which delimiter should be used for textual input formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_delimiter(mut self, delimiter: char) -> Self {
        self.action.options.common.delimiter = Some(delimiter);
        self
    }

    /// Which string should be used in place of a `NULL` value
    /// for textual input formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_null(mut self, null: impl Into<String>) -> Self {
        self.action.options.common.null = Some(null.into());
        self
    }

    /// Which quote character should be used for textual input formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_quote(mut self, quote: char) -> Self {
        self.action.options.common.quote = Some(quote);
        self
    }

    /// Which escape character should be used for textual input formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_escape(mut self, escape: char) -> Self {
        self.action.options.common.escape = Some(escape);
        self
    }

    /// Which string should be used to indicate that
    /// the `default` value should be used in place of that string
    /// for textual formats
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    ///
    /// (This parameter was added with PostgreSQL 16)
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.action.options.default = Some(default.into());
        self
    }

    /// Is a header provided as part of the textual input or not
    ///
    /// See the [PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-copy.html)
    /// for more details.
    pub fn with_header(mut self, header: CopyHeader) -> Self {
        self.action.options.header = Some(header);
        self
    }
}

/// A custom execute function tailored for `COPY FROM` statements
///
/// This trait can be used to execute `COPY FROM` queries constructed
/// via [`copy_from]`
pub trait ExecuteCopyFromDsl<C>
where
    C: Connection<Backend = Pg>,
{
    /// The error type returned by the execute function
    type Error: std::error::Error;

    /// See the trait documentation for details
    fn execute(self, conn: &mut C) -> Result<usize, Self::Error>;
}

#[cfg(feature = "postgres")]
impl<T, A> ExecuteCopyFromDsl<crate::PgConnection> for CopyFromQuery<T, A>
where
    A: CopyFromExpression<T>,
{
    type Error = A::Error;

    fn execute(self, conn: &mut crate::PgConnection) -> Result<usize, A::Error> {
        conn.copy_from::<A, T>(self.action)
    }
}

#[cfg(feature = "r2d2")]
impl<T, A, C> ExecuteCopyFromDsl<crate::r2d2::PooledConnection<crate::r2d2::ConnectionManager<C>>>
    for CopyFromQuery<T, A>
where
    A: CopyFromExpression<T>,
    C: crate::r2d2::R2D2Connection<Backend = Pg> + 'static,
    Self: ExecuteCopyFromDsl<C>,
{
    type Error = <Self as ExecuteCopyFromDsl<C>>::Error;

    fn execute(
        self,
        conn: &mut crate::r2d2::PooledConnection<crate::r2d2::ConnectionManager<C>>,
    ) -> Result<usize, Self::Error> {
        self.execute(&mut **conn)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NotSet;

/// Creates a `COPY FROM` statement
///
/// This function constructs `COPY FROM` statement which copies data
/// *from* a source into the database. It's designed to move larger
/// amounts of data into the database.
///
/// This function accepts a target table as argument.
///
/// There are two ways to construct a `COPY FROM` statement with
/// diesel:
///
/// * By providing a `Vec<I>` where `I` implements `Insertable` for the
///   given table
/// * By providing a target selection (column list or table name)
///   and a callback that provides the data
///
/// The first variant uses the `BINARY` format internally to send
/// the provided data efficiently to the database. It automatically
/// sets the right options and does not allow changing them.
/// Use [`CopyFromQuery::from_insertable`] for this.
///
/// The second variant allows you to control the behaviour
/// of the generated `COPY FROM` statement in detail. It can
/// be setup via the [`CopyFromQuery::from_raw_data`] function.
/// The callback accepts an opaque object as argument that allows
/// to write the corresponding data to the database. The exact
/// format depends on the settings chosen by the various
/// `CopyFromQuery::with_*` methods. See
/// [the postgresql documentation](https://www.postgresql.org/docs/current/sql-copy.html)
/// for more details about the expected formats.
///
/// If you don't have any specific needs you should prefer
/// using the more convenient first variant.
///
/// This functionality is postgresql specific.
///
/// # Examples
///
/// ## Via [`CopyFromQuery::from_insertable`]
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # use crate::schema::users;
///
/// #[derive(Insertable)]
/// #[diesel(table_name = users)]
/// #[diesel(treat_none_as_default_value = false)]
/// struct NewUser {
///     name: &'static str,
/// }
///
/// # fn run_test() -> QueryResult<()> {
/// # let connection = &mut establish_connection();
///
/// let data = vec![
///     NewUser { name: "Diva Plavalaguna" },
///     NewUser { name: "Father Vito Cornelius" },
/// ];
///
/// let count = diesel::copy_from(users::table)
///     .from_insertable(&data)
///     .execute(connection)?;
///
/// assert_eq!(count, 2);
/// # Ok(())
/// # }
/// # fn main() {
/// #    run_test().unwrap();
/// # }
/// ```
///
/// ## Via [`CopyFromQuery::from_raw_data`]
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # fn run_test() -> QueryResult<()> {
/// # use crate::schema::users;
/// use diesel::pg::CopyFormat;
/// # let connection = &mut establish_connection();
/// let count = diesel::copy_from(users::table)
///     .from_raw_data(users::table, |copy| {
///         writeln!(copy, "3,Diva Plavalaguna").unwrap();
///         writeln!(copy, "4,Father Vito Cornelius").unwrap();
///         diesel::QueryResult::Ok(())
///     })
///     .with_format(CopyFormat::Csv)
///     .execute(connection)?;
///
/// assert_eq!(count, 2);
/// # Ok(())
/// # }
/// # fn main() {
/// #    run_test().unwrap();
/// # }
/// ```
#[cfg(feature = "postgres_backend")]
pub fn copy_from<T>(table: T) -> CopyFromQuery<T, NotSet>
where
    T: Table,
{
    CopyFromQuery {
        table,
        action: NotSet,
    }
}
