use std::borrow::Cow;
use std::io::Write;
use std::marker::PhantomData;

use byteorder::NetworkEndian;
use byteorder::WriteBytesExt;

use super::CommonOptions;
use super::CopyFormat;
use super::CopyTarget;
use crate::expression::bound::Bound;
use crate::insertable::ColumnInsertValue;
use crate::pg::backend::FailedToLookupTypeError;
use crate::pg::connection::copy::CopyFromSink;
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
use crate::PgConnection;
use crate::QueryResult;
use crate::{Column, Table};

#[derive(Debug, Copy, Clone)]
pub enum CopyHeader {
    Set(bool),
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
            self.common.walk_ast(pass.reborrow(), &mut comma)?;
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

pub(crate) struct InternalCopyInQuery<S, T> {
    pub(crate) target: S,
    p: PhantomData<T>,
}

impl<S, T> InternalCopyInQuery<S, T> {
    pub(crate) fn new(target: S) -> Self {
        Self {
            target,
            p: PhantomData,
        }
    }
}

impl<S, T> QueryId for InternalCopyInQuery<S, T>
where
    S: CopyInExpression<T>,
{
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<S, T> QueryFragment<Pg> for InternalCopyInQuery<S, T>
where
    S: CopyInExpression<T>,
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
        // todo: where?
        Ok(())
    }
}

pub trait CopyInExpression<T> {
    type Error: From<crate::result::Error> + std::error::Error;

    fn callback(&mut self, copy: &mut CopyFromSink<'_>) -> Result<(), Self::Error>;

    fn walk_target<'b>(
        &'b self,
        pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()>;

    fn options(&self) -> &CopyFromOptions;
}

impl<S, F, E> CopyInExpression<S::Table> for CopyFrom<S, F>
where
    E: From<crate::result::Error> + std::error::Error,
    S: CopyTarget,
    F: Fn(&mut dyn std::io::Write) -> Result<(), E>,
{
    type Error = E;

    fn callback(&mut self, copy: &mut CopyFromSink<'_>) -> Result<(), Self::Error> {
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

impl<I, T, V, QId, const STATIC_QUERY_ID: bool> CopyInExpression<T> for InsertableWrapper<I>
where
    I: Insertable<T, Values = BatchInsert<Vec<V>, T, QId, STATIC_QUERY_ID>>,
    V: CopyFromInsertableHelper,
{
    type Error = crate::result::Error;

    fn callback(&mut self, copy: &mut CopyFromSink<'_>) -> Result<(), Self::Error> {
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

#[derive(Debug)]
pub struct CopyInQuery<T, Action> {
    table: T,
    action: Action,
}

impl<T> CopyInQuery<T, NotSet>
where
    T: Table,
{
    pub fn from_raw_data<F, C, E>(self, _target: C, action: F) -> CopyInQuery<T, CopyFrom<C, F>>
    where
        C: CopyTarget<Table = T>,
        F: Fn(&mut dyn std::io::Write) -> Result<(), E>,
    {
        CopyInQuery {
            table: self.table,
            action: CopyFrom {
                p: PhantomData,
                options: Default::default(),
                copy_callback: action,
            },
        }
    }

    pub fn from_insertable<I>(self, insertable: I) -> CopyInQuery<T, InsertableWrapper<I>>
    where
        InsertableWrapper<I>: CopyInExpression<T>,
    {
        CopyInQuery {
            table: self.table,
            action: InsertableWrapper(Some(insertable)),
        }
    }
}

impl<T, C, F> CopyInQuery<T, CopyFrom<C, F>> {
    pub fn with_format(mut self, format: CopyFormat) -> Self {
        self.action.options.common.format = Some(format);
        self
    }

    pub fn with_freeze(mut self, freeze: bool) -> Self {
        self.action.options.common.freeze = Some(freeze);
        self
    }

    pub fn with_delimiter(mut self, delimiter: char) -> Self {
        self.action.options.common.delimiter = Some(delimiter);
        self
    }

    pub fn with_null(mut self, null: impl Into<String>) -> Self {
        self.action.options.common.null = Some(null.into());
        self
    }

    pub fn with_quote(mut self, quote: char) -> Self {
        self.action.options.common.quote = Some(quote);
        self
    }

    pub fn with_escape(mut self, escape: char) -> Self {
        self.action.options.common.escape = Some(escape);
        self
    }

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.action.options.default = Some(default.into());
        self
    }

    pub fn with_header(mut self, header: CopyHeader) -> Self {
        self.action.options.header = Some(header);
        self
    }
}

pub trait ExecuteCopyInQueryDsl<C>
where
    C: Connection<Backend = Pg>,
{
    type Error: std::error::Error;

    fn execute(self, conn: &mut C) -> Result<usize, Self::Error>;
}

impl<T, A> ExecuteCopyInQueryDsl<PgConnection> for CopyInQuery<T, A>
where
    A: CopyInExpression<T>,
{
    type Error = A::Error;

    fn execute(self, conn: &mut PgConnection) -> Result<usize, A::Error> {
        conn.copy_from::<A, T>(self.action)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NotSet;

pub fn copy_from<T>(table: T) -> CopyInQuery<T, NotSet>
where
    T: Table,
{
    CopyInQuery {
        table,
        action: NotSet,
    }
}
