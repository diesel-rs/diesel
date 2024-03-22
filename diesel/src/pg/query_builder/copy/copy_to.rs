use std::io::BufRead;
use std::marker::PhantomData;

use super::CommonOptions;
use super::CopyFormat;
use super::CopyTarget;
use crate::deserialize::FromSqlRow;
use crate::pg::connection::copy::CopyToBuffer;
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
use crate::AppearsOnTable;
use crate::Connection;
use crate::Expression;
use crate::PgConnection;
use crate::QueryResult;
use crate::Selectable;

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
            self.common.walk_ast(pass.reborrow(), &mut comma)?;
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

pub trait CopyOutMarker: Sized {
    fn setup_options<T>(q: CopyToQuery<T, Self>) -> CopyToQuery<T, CopyToOptions>;
}

impl CopyOutMarker for NotSet {
    fn setup_options<T>(q: CopyToQuery<T, Self>) -> CopyToQuery<T, CopyToOptions> {
        CopyToQuery {
            target: q.target,
            options: CopyToOptions::default(),
        }
    }
}
impl CopyOutMarker for CopyToOptions {
    fn setup_options<T>(q: CopyToQuery<T, Self>) -> CopyToQuery<T, CopyToOptions> {
        q
    }
}

#[derive(Debug)]
pub struct CopyToQuery<T, O> {
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

pub trait ExecuteCopyOutConnection: Connection<Backend = Pg> {
    type CopyOutBuffer<'a>: BufRead;

    fn make_row<'a, 'b>(
        out: &'a Self::CopyOutBuffer<'_>,
        buffers: Vec<Option<&'a [u8]>>,
    ) -> impl Row<'b, Pg> + 'a;

    fn get_buffer<'a>(out: &'a Self::CopyOutBuffer<'_>) -> &'a [u8];

    fn execute<'conn, T>(
        &'conn mut self,
        command: CopyToCommand<T>,
    ) -> QueryResult<Self::CopyOutBuffer<'conn>>
    where
        T: CopyTarget;
}

impl ExecuteCopyOutConnection for PgConnection {
    type CopyOutBuffer<'a> = CopyToBuffer<'a>;

    fn make_row<'a, 'b>(
        out: &'a Self::CopyOutBuffer<'_>,
        buffers: Vec<Option<&'a [u8]>>,
    ) -> impl Row<'b, Pg> + 'a {
        CopyRow {
            buffers,
            result: out.get_result(),
        }
    }

    fn get_buffer<'a>(out: &'a Self::CopyOutBuffer<'_>) -> &'a [u8] {
        out.data_slice()
    }

    fn execute<'conn, T>(
        &'conn mut self,
        command: CopyToCommand<T>,
    ) -> QueryResult<Self::CopyOutBuffer<'conn>>
    where
        T: CopyTarget,
    {
        self.copy_to(command)
    }
}

impl<T> CopyToQuery<T, NotSet>
where
    T: CopyTarget,
{
    pub fn load<'a, U, C>(
        self,
        conn: &'a mut C,
    ) -> QueryResult<impl Iterator<Item = QueryResult<U>> + 'a>
    where
        U: FromSqlRow<<U::SelectExpression as Expression>::SqlType, Pg> + Selectable<Pg>,
        U::SelectExpression: AppearsOnTable<T::Table> + CopyTarget<Table = T::Table>,
        C: ExecuteCopyOutConnection,
    {
        let io_result_mapper = |e| crate::result::Error::DeserializationError(Box::new(e));

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
        // see https://www.postgresql.org/docs/current/sql-copy.html for
        // a description of the binary format
        //
        // We don't write oids

        let mut out = ExecuteCopyOutConnection::execute(conn, command)?;
        let buffer = out.fill_buf().map_err(io_result_mapper)?;
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
            let buffer = C::get_buffer(&out);
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

                let row = C::make_row(&out, buffers);
                Some(U::build_from_row(&row).map_err(crate::result::Error::DeserializationError))
            } else {
                None
            }
        }))
    }
}

impl<T, O> CopyToQuery<T, O>
where
    O: CopyOutMarker,
    T: CopyTarget,
{
    pub fn load_raw<'conn, C>(self, conn: &'conn mut C) -> QueryResult<impl BufRead + 'conn>
    where
        C: ExecuteCopyOutConnection,
    {
        let q = O::setup_options(self);
        let command = CopyToCommand {
            p: PhantomData::<T>,
            options: q.options,
        };
        ExecuteCopyOutConnection::execute(conn, command)
    }

    pub fn with_format(self, format: CopyFormat) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.format = Some(format);
        out
    }

    pub fn with_freeze(self, freeze: bool) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.freeze = Some(freeze);
        out
    }

    pub fn with_delimiter(self, delimiter: char) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.delimiter = Some(delimiter);
        out
    }

    pub fn with_null(self, null: impl Into<String>) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.null = Some(null.into());
        out
    }

    pub fn with_quote(self, quote: char) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.quote = Some(quote);
        out
    }

    pub fn with_escape(self, escape: char) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.common.escape = Some(escape);
        out
    }

    pub fn with_header(self, set: bool) -> CopyToQuery<T, CopyToOptions> {
        let mut out = O::setup_options(self);
        out.options.header = Some(set);
        out
    }
}

pub fn copy_to<T>(target: T) -> CopyToQuery<T, NotSet> {
    CopyToQuery {
        target,
        options: NotSet,
    }
}
