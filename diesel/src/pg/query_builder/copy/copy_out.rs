use std::io::BufRead;
use std::marker::PhantomData;

use super::CommonOptions;
use super::CopyFormat;
use super::CopyTarget;
use crate::deserialize::FromSqlRow;
use crate::pg::connection::copy::CopyOut;
use crate::pg::connection::PgResult;
use crate::pg::value::TypeOidLookup;
use crate::pg::Pg;
use crate::pg::PgValue;
use crate::query_builder::QueryFragment;
use crate::query_builder::QueryId;
use crate::row;
use crate::row::Field;
use crate::row::PartialRow;
use crate::row::Row;
use crate::row::RowIndex;
use crate::row::RowSealed;
use crate::PgConnection;
use crate::QueryResult;

#[derive(Default, Debug)]
pub struct CopyToOptions {
    common: CommonOptions,
}

impl CopyToOptions {
    fn any_set(&self) -> bool {
        self.common.any_set()
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
            self.common.walk_ast(pass.reborrow(), &mut comma)?;

            pass.push_sql(")");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct CopyTo<S> {
    options: CopyToOptions,
    p: PhantomData<S>,
}

impl<S> QueryId for CopyTo<S>
where
    S: CopyTarget,
{
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<S> QueryFragment<Pg> for CopyTo<S>
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

pub trait CopyOutMarker: Sized {
    fn setup_options<T>(q: CopyOutQuery<T, Self>) -> CopyOutQuery<T, CopyToOptions>;
}

impl CopyOutMarker for NotSet {
    fn setup_options<T>(q: CopyOutQuery<T, Self>) -> CopyOutQuery<T, CopyToOptions> {
        CopyOutQuery {
            target: q.target,
            options: CopyToOptions::default(),
        }
    }
}
impl CopyOutMarker for CopyToOptions {
    fn setup_options<T>(q: CopyOutQuery<T, Self>) -> CopyOutQuery<T, CopyToOptions> {
        q
    }
}

#[derive(Debug)]
pub struct CopyOutQuery<T, O> {
    target: T,
    options: O,
}

struct CopyRow<'a> {
    buffers: Vec<Option<&'a [u8]>>,
    result: &'a PgResult,
}

struct CopyField<'a> {
    field: &'a Option<&'a [u8]>,
    result: &'a PgResult,
    col_idx: usize,
}

impl<'f> Field<'f, Pg> for CopyField<'f> {
    fn field_name(&self) -> Option<&str> {
        None
    }

    fn value(&self) -> Option<<Pg as crate::backend::Backend>::RawValue<'_>> {
        let value = self.field.as_deref()?;
        Some(PgValue::new_internal(value, self))
    }
}

impl<'a> TypeOidLookup for CopyField<'a> {
    fn lookup(&self) -> std::num::NonZeroU32 {
        self.result.column_type(self.col_idx)
    }
}

impl RowSealed for CopyRow<'_> {}

impl RowIndex<usize> for CopyRow<'_> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a> RowIndex<&'a str> for CopyRow<'_> {
    fn idx(&self, _idx: &'a str) -> Option<usize> {
        None
    }
}

impl<'a> Row<'a, Pg> for CopyRow<'_> {
    type Field<'f> = CopyField<'f>
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
        range: std::ops::Range<usize>,
    ) -> row::PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<T> CopyOutQuery<T, NotSet>
where
    T: CopyTarget,
{
    pub fn load<'a, U>(
        self,
        conn: &'a mut PgConnection,
    ) -> QueryResult<impl Iterator<Item = QueryResult<U>> + 'a>
    where
        U: FromSqlRow<T::SqlType, Pg>,
    {
        let io_result_mapper = |e| crate::result::Error::DeserializationError(Box::new(e));

        let command = CopyTo {
            p: PhantomData::<T>,
            options: CopyToOptions {
                common: CommonOptions {
                    format: Some(CopyFormat::Binary),
                    ..Default::default()
                },
            },
        };
        // see https://www.postgresql.org/docs/current/sql-copy.html for
        // a description of the binary format
        //
        // We don't write oids

        let mut out = conn.copy_out(command)?;
        out.fill_buf().map_err(io_result_mapper)?;
        let buffer = out.data_slice();
        if &buffer[..super::COPY_MAGIC_HEADER.len()] != super::COPY_MAGIC_HEADER {
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
        let header_size = i32::from_be_bytes(
            (&buffer[super::COPY_MAGIC_HEADER.len() + 4..super::COPY_MAGIC_HEADER.len() + 8])
                .try_into()
                .expect("Exactly 4 byte"),
        );
        out.consume(super::COPY_MAGIC_HEADER.len() + 8 + header_size as usize);
        let mut len = None;
        Ok(std::iter::from_fn(move || {
            if let Some(len) = len {
                out.consume(len);
                if let Err(e) = out.fill_buf().map_err(io_result_mapper) {
                    return Some(Err(e));
                }
            }
            let buffer = out.data_slice();
            len = Some(buffer.len());
            let tuple_count =
                i16::from_be_bytes((&buffer[..2]).try_into().expect("Exactly 2 bytes"));
            if tuple_count > 0 {
                let mut buffers = Vec::with_capacity(tuple_count as usize);
                let mut offset = 2;
                for _t in 0..tuple_count {
                    let data_size = i32::from_be_bytes(
                        (&buffer[offset..offset + 4])
                            .try_into()
                            .expect("Exactly 4 bytes"),
                    );
                    if data_size < 0 {
                        buffers.push(None);
                    } else {
                        buffers.push(Some(&buffer[offset + 4..offset + 4 + data_size as usize]));
                        offset = offset + 4 + data_size as usize;
                    }
                }
                let row = CopyRow {
                    buffers,
                    result: out.get_result(),
                };
                Some(U::build_from_row(&row).map_err(crate::result::Error::DeserializationError))
            } else {
                None
            }
        }))
    }
}

impl<T, O> CopyOutQuery<T, O>
where
    O: CopyOutMarker,
    T: CopyTarget,
{
    pub fn load_raw(self, conn: &mut PgConnection) -> QueryResult<CopyOut<'_>> {
        let q = O::setup_options(self);
        let command = CopyTo {
            p: PhantomData::<T>,
            options: q.options,
        };
        conn.copy_out(command)
    }

    pub fn with_format(self, format: CopyFormat) -> CopyOutQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.format = Some(format);
        out
    }

    pub fn with_freeze(self, freeze: bool) -> CopyOutQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.freeze = Some(freeze);
        out
    }

    pub fn with_delimiter(self, delimiter: char) -> CopyOutQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.delimiter = Some(delimiter);
        out
    }

    pub fn with_null(self, null: impl Into<String>) -> CopyOutQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.null = Some(null.into());
        out
    }

    pub fn with_quote(self, quote: char) -> CopyOutQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.quote = Some(quote);
        out
    }

    pub fn with_escape(self, escape: char) -> CopyOutQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.escape = Some(escape);
        out
    }
}

pub fn copy_out<T>(target: T) -> CopyOutQuery<T, NotSet> {
    CopyOutQuery {
        target,
        options: NotSet,
    }
}
