use crate::QueryResult;
use crate::query_builder::{BindCollector, MoveableBindCollector};
use crate::serialize::{IsNull, Output};
use crate::sql_types::HasSqlType;
use crate::sqlite::{Sqlite, SqliteType};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use libsqlite3_sys as ffi;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

/// The [`BindCollector`] used by the SQLite backend.
///
/// You only interact with this when adding a third-party SQLite backend.
#[derive(Debug, Default)]
pub struct SqliteBindCollector<'a> {
    pub(in crate::sqlite) binds: Vec<(SqliteBindValueRef<'a>, SqliteType)>,
}

impl<'a> SqliteBindCollector<'a> {
    /// Construct an empty `SqliteBindCollector`
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub(in crate::sqlite) fn new() -> Self {
        Self { binds: Vec::new() }
    }

    /// Iterate over the collected bind values and their SQLite storage classes,
    /// in positional order.
    ///
    /// Each yielded tuple carries a reference to the [`SqliteBindValueRef`] as
    /// it lives inside the collector, so borrowed string and blob variants are
    /// exposed without an intervening copy. If the caller needs a
    /// [`Send`] snapshot instead, use [`MoveableBindCollector::moveable`]
    /// and inspect [`SqliteBindCollectorData`].
    ///
    /// # Example
    ///
    /// A third-party backend can render a typed Diesel query into
    /// placeholder SQL and recover the ordered bind values:
    ///
    /// ```rust
    /// use diesel::expression::IntoSql;
    /// use diesel::query_builder::{QueryBuilder, QueryFragment};
    /// use diesel::sql_types::{BigInt, Integer, Text};
    /// use diesel::sqlite::{
    ///     Sqlite, SqliteBindCollector, SqliteBindValueRef, SqliteQueryBuilder, SqliteType,
    /// };
    ///
    /// fn render<Q: QueryFragment<Sqlite>>(query: &Q) -> (String, Vec<SqliteType>) {
    ///     let mut qb = SqliteQueryBuilder::new();
    ///     query.to_sql(&mut qb, &Sqlite).unwrap();
    ///
    ///     let mut collector = SqliteBindCollector::new();
    ///     query.collect_binds(&mut collector, &mut (), &Sqlite).unwrap();
    ///
    ///     let binds: Vec<(&SqliteBindValueRef<'_>, SqliteType)> = collector.binds().collect();
    ///     assert!(matches!(binds[0].0, SqliteBindValueRef::I32(1)));
    ///     assert!(matches!(binds[1].0, SqliteBindValueRef::I64(2)));
    ///     assert!(matches!(binds[2].0, SqliteBindValueRef::BorrowedString("hi")));
    ///
    ///     (qb.finish(), binds.iter().map(|(_, t)| *t).collect())
    /// }
    ///
    /// let query = diesel::select((
    ///     1_i32.into_sql::<Integer>(),
    ///     2_i64.into_sql::<BigInt>(),
    ///     "hi".into_sql::<Text>(),
    /// ));
    ///
    /// let (sql, types) = render(&query);
    /// assert!(sql.contains('?'));
    /// assert_eq!(types, [SqliteType::Integer, SqliteType::Long, SqliteType::Text]);
    /// ```
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn binds(&self) -> impl ExactSizeIterator<Item = (&SqliteBindValueRef<'a>, SqliteType)> {
        self.binds.iter().map(|(v, t)| (v, *t))
    }
}

/// This type represents a value bound to
/// a sqlite prepared statement
///
/// It can be constructed via the various `From<T>` implementations
#[derive(Debug)]
pub struct SqliteBindValue<'a> {
    pub(in crate::sqlite) inner: SqliteBindValueRef<'a>,
}

impl From<i32> for SqliteBindValue<'_> {
    fn from(i: i32) -> Self {
        Self {
            inner: SqliteBindValueRef::I32(i),
        }
    }
}

impl From<i64> for SqliteBindValue<'_> {
    fn from(i: i64) -> Self {
        Self {
            inner: SqliteBindValueRef::I64(i),
        }
    }
}

impl From<f64> for SqliteBindValue<'_> {
    fn from(f: f64) -> Self {
        Self {
            inner: SqliteBindValueRef::F64(f),
        }
    }
}

impl<'a, T> From<Option<T>> for SqliteBindValue<'a>
where
    T: Into<SqliteBindValue<'a>>,
{
    fn from(o: Option<T>) -> Self {
        match o {
            Some(v) => v.into(),
            None => Self {
                inner: SqliteBindValueRef::Null,
            },
        }
    }
}

impl<'a> From<&'a str> for SqliteBindValue<'a> {
    fn from(s: &'a str) -> Self {
        Self {
            inner: SqliteBindValueRef::BorrowedString(s),
        }
    }
}

impl From<String> for SqliteBindValue<'_> {
    fn from(s: String) -> Self {
        Self {
            inner: SqliteBindValueRef::String(s.into_boxed_str()),
        }
    }
}

impl From<Vec<u8>> for SqliteBindValue<'_> {
    fn from(b: Vec<u8>) -> Self {
        Self {
            inner: SqliteBindValueRef::Binary(b.into_boxed_slice()),
        }
    }
}

impl<'a> From<&'a [u8]> for SqliteBindValue<'a> {
    fn from(b: &'a [u8]) -> Self {
        Self {
            inner: SqliteBindValueRef::BorrowedBinary(b),
        }
    }
}

/// The concrete bind value carried by a live [`SqliteBindCollector`].
///
/// Distinct from [`OwnedSqliteBindValue`] (the moved snapshot stored in
/// [`SqliteBindCollectorData`]) in that borrowed and owned string or blob
/// variants are kept separate, which lets third-party backends read the
/// collector without cloning transient buffers.
#[derive(Debug)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) enum SqliteBindValueRef<'a> {
    /// A `TEXT` value that borrows from the query.
    BorrowedString(&'a str),
    /// A `TEXT` value the collector owns.
    String(Box<str>),
    /// A `BLOB` value that borrows from the query.
    BorrowedBinary(&'a [u8]),
    /// A `BLOB` value the collector owns.
    Binary(Box<[u8]>),
    /// An `INTEGER` value that fits in an `i32`.
    I32(i32),
    /// An `INTEGER` value that requires an `i64`.
    I64(i64),
    /// A `REAL` value.
    F64(f64),
    /// A `NULL` value.
    Null,
}

impl core::fmt::Display for SqliteBindValueRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let n = match self {
            SqliteBindValueRef::BorrowedString(_) | SqliteBindValueRef::String(_) => "Text",
            SqliteBindValueRef::BorrowedBinary(_) | SqliteBindValueRef::Binary(_) => "Binary",
            SqliteBindValueRef::I32(_) | SqliteBindValueRef::I64(_) => "Integer",
            SqliteBindValueRef::F64(_) => "Float",
            SqliteBindValueRef::Null => "Null",
        };
        f.write_str(n)
    }
}

impl SqliteBindValueRef<'_> {
    #[allow(unsafe_code)] // ffi function calls
    pub(in crate::sqlite) fn result_of(
        self,
        ctx: &mut ffi::sqlite3_context,
    ) -> Result<(), core::num::TryFromIntError> {
        use core::ffi as libc;
        // This unsafe block assumes the following invariants:
        //
        // - `ctx` points to valid memory
        unsafe {
            match self {
                SqliteBindValueRef::BorrowedString(s) => ffi::sqlite3_result_text(
                    ctx,
                    s.as_ptr() as *const libc::c_char,
                    s.len().try_into()?,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValueRef::String(s) => ffi::sqlite3_result_text(
                    ctx,
                    s.as_ptr() as *const libc::c_char,
                    s.len().try_into()?,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValueRef::Binary(b) => ffi::sqlite3_result_blob(
                    ctx,
                    b.as_ptr() as *const libc::c_void,
                    b.len().try_into()?,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValueRef::BorrowedBinary(b) => ffi::sqlite3_result_blob(
                    ctx,
                    b.as_ptr() as *const libc::c_void,
                    b.len().try_into()?,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValueRef::I32(i) => ffi::sqlite3_result_int(ctx, i as libc::c_int),
                SqliteBindValueRef::I64(l) => ffi::sqlite3_result_int64(ctx, l),
                SqliteBindValueRef::F64(d) => ffi::sqlite3_result_double(ctx, d as libc::c_double),
                SqliteBindValueRef::Null => ffi::sqlite3_result_null(ctx),
            }
        }
        Ok(())
    }
}

impl<'a> BindCollector<'a, Sqlite> for SqliteBindCollector<'a> {
    type Buffer = SqliteBindValue<'a>;

    fn push_bound_value<T, U>(&mut self, bind: &'a U, metadata_lookup: &mut ()) -> QueryResult<()>
    where
        Sqlite: crate::sql_types::HasSqlType<T>,
        U: crate::serialize::ToSql<T, Sqlite> + ?Sized,
    {
        let value = SqliteBindValue {
            inner: SqliteBindValueRef::Null,
        };
        let mut to_sql_output = Output::new(value, metadata_lookup);
        let is_null = bind
            .to_sql(&mut to_sql_output)
            .map_err(crate::result::Error::SerializationError)?;
        let bind = to_sql_output.into_inner();
        let metadata = Sqlite::metadata(metadata_lookup);
        self.binds.push((
            match is_null {
                IsNull::No => bind.inner,
                IsNull::Yes => SqliteBindValueRef::Null,
            },
            metadata,
        ));
        Ok(())
    }

    fn push_null_value(&mut self, metadata: SqliteType) -> QueryResult<()> {
        self.binds.push((SqliteBindValueRef::Null, metadata));
        Ok(())
    }
}

/// An owned value bound to a SQLite prepared statement.
///
/// The readable counterpart to the values a [`SqliteBindCollector`] holds.
#[derive(Debug, Clone)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
enum OwnedSqliteBindValue {
    /// A `TEXT` value.
    String(Box<str>),
    /// A `BLOB` value.
    Binary(Box<[u8]>),
    /// An `INTEGER` value that fits in an `i32`.
    I32(i32),
    /// An `INTEGER` value that requires an `i64`.
    I64(i64),
    /// A `REAL` value.
    F64(f64),
    /// A `NULL` value.
    Null,
}

impl<'a> core::convert::From<&SqliteBindValueRef<'a>> for OwnedSqliteBindValue {
    fn from(value: &SqliteBindValueRef<'a>) -> Self {
        match value {
            SqliteBindValueRef::String(s) => Self::String(s.clone()),
            SqliteBindValueRef::BorrowedString(s) => {
                Self::String(String::from(*s).into_boxed_str())
            }
            SqliteBindValueRef::Binary(b) => Self::Binary(b.clone()),
            SqliteBindValueRef::BorrowedBinary(s) => Self::Binary(Vec::from(*s).into_boxed_slice()),
            SqliteBindValueRef::I32(val) => Self::I32(*val),
            SqliteBindValueRef::I64(val) => Self::I64(*val),
            SqliteBindValueRef::F64(val) => Self::F64(*val),
            SqliteBindValueRef::Null => Self::Null,
        }
    }
}

impl core::convert::From<&OwnedSqliteBindValue> for SqliteBindValueRef<'_> {
    fn from(value: &OwnedSqliteBindValue) -> Self {
        match value {
            OwnedSqliteBindValue::String(s) => Self::String(s.clone()),
            OwnedSqliteBindValue::Binary(b) => Self::Binary(b.clone()),
            OwnedSqliteBindValue::I32(val) => Self::I32(*val),
            OwnedSqliteBindValue::I64(val) => Self::I64(*val),
            OwnedSqliteBindValue::F64(val) => Self::F64(*val),
            OwnedSqliteBindValue::Null => Self::Null,
        }
    }
}

/// SQLite bind collector data that is movable across threads.
///
/// This is the [`Send`] snapshot produced by [`MoveableBindCollector::moveable`]
/// on a [`SqliteBindCollector`]. Both borrowed and owned string or blob variants
/// of [`SqliteBindValueRef`] collapse into their [`OwnedSqliteBindValue`]
/// counterparts, so a caller crossing a thread boundary carries no borrows from
/// the original query. For a zero-copy view of the live collector see
/// [`SqliteBindCollector::binds`].
#[derive(Debug)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub struct SqliteBindCollectorData {
    /// The collected bind values, in the order they appear in the query.
    binds: Vec<(OwnedSqliteBindValue, SqliteType)>,
}

impl SqliteBindCollectorData {
    /// Iterate over the collected bind values and their SQLite storage classes,
    /// in positional order.
    ///
    /// The values yielded are the moved [`OwnedSqliteBindValue`] snapshot, so
    /// this method is safe to call from any thread. For a zero-copy view of the
    /// live collector see [`SqliteBindCollector::binds`].
    ///
    /// # Example
    ///
    /// A third-party backend can snapshot the ordered bind values of a typed
    /// Diesel query and inspect the owned variants:
    ///
    /// ```rust
    /// use diesel::expression::IntoSql;
    /// use diesel::query_builder::{MoveableBindCollector, QueryFragment};
    /// use diesel::sql_types::{Binary, Integer, Nullable, Text};
    /// use diesel::sqlite::{
    ///     OwnedSqliteBindValue, Sqlite, SqliteBindCollector, SqliteBindCollectorData, SqliteType,
    /// };
    ///
    /// fn snapshot<Q: QueryFragment<Sqlite>>(query: &Q) -> SqliteBindCollectorData {
    ///     let mut collector = SqliteBindCollector::new();
    ///     query.collect_binds(&mut collector, &mut (), &Sqlite).unwrap();
    ///     collector.moveable()
    /// }
    ///
    /// let query = diesel::select((
    ///     42_i32.into_sql::<Integer>(),
    ///     "hi".into_sql::<Text>(),
    ///     vec![1_u8, 2, 3].into_sql::<Binary>(),
    ///     None::<i32>.into_sql::<Nullable<Integer>>(),
    /// ));
    ///
    /// let data = snapshot(&query);
    ///
    /// // Types survive the move.
    /// let types: Vec<_> = data.binds().map(|(_, t)| t).collect();
    /// assert_eq!(
    ///     types,
    ///     [SqliteType::Integer, SqliteType::Text, SqliteType::Binary, SqliteType::Integer],
    /// );
    ///
    /// // Borrowed variants normalize into their owned counterparts.
    /// let binds: Vec<_> = data.binds().collect();
    /// assert!(matches!(binds[0].0, OwnedSqliteBindValue::I32(42)));
    /// assert!(matches!(binds[1].0, OwnedSqliteBindValue::String(s) if &**s == "hi"));
    /// assert!(matches!(binds[2].0, OwnedSqliteBindValue::Binary(b) if **b == [1, 2, 3]));
    /// assert!(matches!(binds[3].0, OwnedSqliteBindValue::Null));
    /// ```
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn binds(&self) -> impl ExactSizeIterator<Item = (&OwnedSqliteBindValue, SqliteType)> {
        self.binds.iter().map(|(v, t)| (v, *t))
    }
}

impl MoveableBindCollector<Sqlite> for SqliteBindCollector<'_> {
    type BindData = SqliteBindCollectorData;

    fn moveable(&self) -> Self::BindData {
        let mut binds = Vec::with_capacity(self.binds.len());
        for b in self
            .binds
            .iter()
            .map(|(bind, tpe)| (OwnedSqliteBindValue::from(bind), *tpe))
        {
            binds.push(b);
        }
        SqliteBindCollectorData { binds }
    }

    fn append_bind_data(&mut self, from: &Self::BindData) {
        self.binds.reserve_exact(from.binds.len());
        self.binds.extend(
            from.binds
                .iter()
                .map(|(bind, tpe)| (SqliteBindValueRef::from(bind), *tpe)),
        );
    }

    fn push_debug_binds<'a, 'b>(
        bind_data: &Self::BindData,
        f: &'a mut Vec<Box<dyn core::fmt::Debug + 'b>>,
    ) {
        f.extend(
            bind_data
                .binds
                .iter()
                .map(|(b, _)| Box::new(b.clone()) as Box<dyn core::fmt::Debug>),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OwnedSqliteBindValue, SqliteBindCollector, SqliteBindCollectorData, SqliteBindValueRef,
    };
    use crate::expression::IntoSql;
    use crate::query_builder::{MoveableBindCollector, QueryFragment};
    use crate::sql_types::{BigInt, Binary, Double, Integer, Nullable, Text};
    use crate::sqlite::{Sqlite, SqliteType};

    // Collecting binds needs no connection, the property downstream callers rely on.
    fn collect<Q: QueryFragment<Sqlite>>(query: Q) -> SqliteBindCollectorData {
        let mut collector = SqliteBindCollector::new();
        query
            .collect_binds(&mut collector, &mut (), &Sqlite)
            .unwrap();
        collector.moveable()
    }

    #[diesel_test_helper::test]
    fn collected_binds_are_readable_in_positional_order_with_their_type() {
        let data = collect(crate::select((
            1_i32.into_sql::<Integer>(),
            2_i64.into_sql::<BigInt>(),
            3.5_f64.into_sql::<Double>(),
            "hello".into_sql::<Text>(),
            vec![1_u8, 2, 3].into_sql::<Binary>(),
            None::<i32>.into_sql::<Nullable<Integer>>(),
        )));

        let types: Vec<_> = data.binds.iter().map(|(_, t)| *t).collect();
        assert_eq!(
            types,
            [
                SqliteType::Integer,
                SqliteType::Long,
                SqliteType::Double,
                SqliteType::Text,
                SqliteType::Binary,
                SqliteType::Integer,
            ]
        );

        assert!(matches!(data.binds[0].0, OwnedSqliteBindValue::I32(1)));
        assert!(matches!(data.binds[1].0, OwnedSqliteBindValue::I64(2)));
        assert!(matches!(data.binds[2].0, OwnedSqliteBindValue::F64(f) if f == 3.5));
        assert!(matches!(&data.binds[3].0, OwnedSqliteBindValue::String(s) if &**s == "hello"));
        assert!(matches!(&data.binds[4].0, OwnedSqliteBindValue::Binary(b) if **b == [1, 2, 3]));
        assert!(matches!(data.binds[5].0, OwnedSqliteBindValue::Null));
    }

    // `moveable` must own every internal variant, borrowed and owned alike.
    #[diesel_test_helper::test]
    fn moveable_owns_every_internal_variant() {
        let collector = SqliteBindCollector {
            binds: vec![
                (
                    SqliteBindValueRef::BorrowedString("borrowed"),
                    SqliteType::Text,
                ),
                (SqliteBindValueRef::String("owned".into()), SqliteType::Text),
                (
                    SqliteBindValueRef::BorrowedBinary(&[1, 2]),
                    SqliteType::Binary,
                ),
                (
                    SqliteBindValueRef::Binary(vec![3, 4].into()),
                    SqliteType::Binary,
                ),
                (SqliteBindValueRef::I32(7), SqliteType::Integer),
                (SqliteBindValueRef::I64(8), SqliteType::Long),
                (SqliteBindValueRef::F64(9.0), SqliteType::Double),
                (SqliteBindValueRef::Null, SqliteType::Text),
            ],
        };

        let data = collector.moveable();
        assert!(matches!(&data.binds[0].0, OwnedSqliteBindValue::String(s) if &**s == "borrowed"));
        assert!(matches!(&data.binds[1].0, OwnedSqliteBindValue::String(s) if &**s == "owned"));
        assert!(matches!(&data.binds[2].0, OwnedSqliteBindValue::Binary(b) if **b == [1, 2]));
        assert!(matches!(&data.binds[3].0, OwnedSqliteBindValue::Binary(b) if **b == [3, 4]));
        assert!(matches!(data.binds[4].0, OwnedSqliteBindValue::I32(7)));
        assert!(matches!(data.binds[5].0, OwnedSqliteBindValue::I64(8)));
        assert!(matches!(data.binds[6].0, OwnedSqliteBindValue::F64(f) if f == 9.0));
        assert!(matches!(data.binds[7].0, OwnedSqliteBindValue::Null));
    }

    // `SqliteBindCollector::binds` exposes the live enum without cloning
    // borrowed variants, which is the reason it exists next to `moveable()`.
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    #[diesel_test_helper::test]
    fn binds_iterator_yields_live_ref_without_cloning() {
        let collector = SqliteBindCollector {
            binds: vec![
                (
                    SqliteBindValueRef::BorrowedString("borrowed"),
                    SqliteType::Text,
                ),
                (
                    SqliteBindValueRef::BorrowedBinary(&[9, 9]),
                    SqliteType::Binary,
                ),
                (SqliteBindValueRef::I32(1), SqliteType::Integer),
                (SqliteBindValueRef::Null, SqliteType::Text),
            ],
        };

        let seen: Vec<_> = collector.binds().collect();
        assert_eq!(seen.len(), 4);
        assert_eq!(seen[0].1, SqliteType::Text);
        assert_eq!(seen[1].1, SqliteType::Binary);
        assert!(matches!(
            seen[0].0,
            SqliteBindValueRef::BorrowedString("borrowed")
        ));
        assert!(matches!(seen[1].0, SqliteBindValueRef::BorrowedBinary(b) if *b == [9, 9]));
        assert!(matches!(seen[2].0, SqliteBindValueRef::I32(1)));
        assert!(matches!(seen[3].0, SqliteBindValueRef::Null));
    }

    // Appending an owned snapshot back into a collector, the reverse conversion.
    #[diesel_test_helper::test]
    fn append_bind_data_round_trips_the_owned_snapshot() {
        let data = collect(crate::select((
            42_i32.into_sql::<Integer>(),
            "text".into_sql::<Text>(),
            None::<i32>.into_sql::<Nullable<Integer>>(),
        )));

        let mut collector = SqliteBindCollector::new();
        collector.append_bind_data(&data);
        let round_tripped = collector.moveable();

        assert!(matches!(
            round_tripped.binds[0].0,
            OwnedSqliteBindValue::I32(42)
        ));
        assert!(
            matches!(&round_tripped.binds[1].0, OwnedSqliteBindValue::String(s) if &**s == "text")
        );
        assert!(matches!(
            round_tripped.binds[2].0,
            OwnedSqliteBindValue::Null
        ));
    }
}
