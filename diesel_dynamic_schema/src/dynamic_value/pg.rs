//! Dynamic values for PostgreSQL.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::convert::{TryFrom, TryInto};
use core::num::NonZeroU32;

use diesel::deserialize::{self, FromSql};
use diesel::pg::data_types::{PgDate, PgInterval, PgLsn, PgMoney, PgNumeric, PgTime, PgTimestamp};
use diesel::pg::{Pg, PgValue};
use diesel::row::Field;
use diesel::sql_types as st;

use super::{
    BackendDynamicValue, DynamicColumnOrigin, DynamicDecodeContext, DynamicValue,
    DynamicValueBackend, DynamicValueExtension,
};

const BOOL: u32 = 16;
const BYTEA: u32 = 17;
const CCHAR: u32 = 18;
const NAME: u32 = 19;
const INT8: u32 = 20;
const INT2: u32 = 21;
const INT4: u32 = 23;
const TEXT: u32 = 25;
const OID: u32 = 26;
const JSON: u32 = 114;
const FLOAT4: u32 = 700;
const FLOAT8: u32 = 701;
const MONEY: u32 = 790;
const MACADDR8: u32 = 774;
const MACADDR: u32 = 829;
const BPCHAR: u32 = 1042;
const VARCHAR: u32 = 1043;
const DATE: u32 = 1082;
const TIME: u32 = 1083;
const TIMESTAMP: u32 = 1114;
const TIMESTAMPTZ: u32 = 1184;
const INTERVAL: u32 = 1186;
const NUMERIC: u32 = 1700;
const UUID: u32 = 2950;
const PGLSN: u32 = 3220;
const JSONB: u32 = 3802;

/// The runtime type of a PostgreSQL value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PgTypeTag {
    /// A known scalar OID.
    Scalar(NonZeroU32),
    /// A known array OID.
    Array(NonZeroU32),
    /// An unclassified OID.
    Other(NonZeroU32),
}

/// A PostgreSQL backend-only dynamic value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PgBackendValue<E> {
    /// A `DATE`.
    Date(PgDate),
    /// A `TIME`.
    Time(PgTime),
    /// A `TIMESTAMP`.
    Timestamp(PgTimestamp),
    /// A `TIMESTAMPTZ`.
    Timestamptz(PgTimestamp),
    /// An `INTERVAL`.
    Interval(PgInterval),
    /// A `NUMERIC`.
    Numeric(PgNumeric),
    /// A `MONEY`.
    Money(PgMoney),
    /// A `UUID`.
    Uuid([u8; 16]),
    /// A `JSON` document.
    Json(String),
    /// A `JSONB` document.
    Jsonb(String),
    /// A `MACADDR`.
    MacAddr([u8; 6]),
    /// A `MACADDR8`.
    MacAddr8([u8; 8]),
    /// An `OID`.
    Oid(u32),
    /// A PostgreSQL `"char"`.
    CChar(u8),
    /// A `PG_LSN`.
    PgLsn(PgLsn),
    /// A PostgreSQL array.
    Array(PgDynamicArray<E>),
    /// A value with an unknown OID.
    Opaque {
        /// The PostgreSQL type OID.
        oid: NonZeroU32,
        /// The raw wire bytes.
        bytes: Vec<u8>,
    },
}

/// A PostgreSQL array dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgArrayDimension {
    /// The dimension length.
    pub length: i32,
    /// The PostgreSQL lower bound.
    pub lower_bound: i32,
}

/// A PostgreSQL dynamic array.
#[derive(Debug, Clone, PartialEq)]
pub struct PgDynamicArray<E> {
    dimensions: Vec<PgArrayDimension>,
    values: Vec<BackendDynamicValue<Pg, E>>,
}

impl<E> PgDynamicArray<E> {
    /// Return array dimensions.
    pub fn dimensions(&self) -> &[PgArrayDimension] {
        &self.dimensions
    }

    /// Return flattened values in wire order.
    pub fn values(&self) -> &[BackendDynamicValue<Pg, E>] {
        &self.values
    }

    /// Return an iterator over flattened values.
    pub fn iter(&self) -> impl Iterator<Item = &BackendDynamicValue<Pg, E>> {
        self.values.iter()
    }

    /// Return a value by PostgreSQL subscripts.
    pub fn get(&self, subscripts: &[i32]) -> Option<&BackendDynamicValue<Pg, E>> {
        if subscripts.len() != self.dimensions.len() {
            return None;
        }

        let mut index = 0usize;
        for (dimension, subscript) in self.dimensions.iter().zip(subscripts) {
            let upper = dimension.lower_bound.checked_add(dimension.length)?;
            if *subscript < dimension.lower_bound || *subscript >= upper {
                return None;
            }
            let length = usize::try_from(dimension.length).ok()?;
            let offset = usize::try_from(*subscript - dimension.lower_bound).ok()?;
            index = index.checked_mul(length)?.checked_add(offset)?;
        }
        self.values.get(index)
    }
}

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
impl super::private::DynamicValueBackendSeal for Pg {}

impl DynamicValueBackend for Pg {
    type BackendValue<E> = PgBackendValue<E>;
    type TypeTag = PgTypeTag;

    fn dynamic_type_tag(value: PgValue<'_>) -> Self::TypeTag {
        type_tag(value.get_oid())
    }

    fn decode_dynamic_value<'a, F, Ext>(
        field: &F,
        context: &DynamicDecodeContext<'_, Self>,
        extension: &Ext,
    ) -> deserialize::Result<BackendDynamicValue<Self, Ext::Value>>
    where
        F: Field<'a, Self>,
        Ext: DynamicValueExtension<Self>,
    {
        let value = field.value().ok_or(diesel::result::UnexpectedNullError)?;
        decode_pg_value(
            value,
            context.field_name(),
            context.origin(),
            context.declared_sql_type(),
            context.backend_tag(),
            context.pg_array_subscripts(),
            extension,
            false,
        )
    }
}

/// Classify a PostgreSQL type OID.
pub fn type_tag(oid: NonZeroU32) -> PgTypeTag {
    match oid.get() {
        BOOL | BYTEA | CCHAR | NAME | INT8 | INT2 | INT4 | TEXT | OID | JSON | FLOAT4 | FLOAT8
        | MONEY | MACADDR8 | MACADDR | BPCHAR | VARCHAR | DATE | TIME | TIMESTAMP | TIMESTAMPTZ
        | INTERVAL | NUMERIC | UUID | PGLSN | JSONB => PgTypeTag::Scalar(oid),
        199 | 775 | 791 | 1000 | 1001 | 1002 | 1003 | 1005 | 1007 | 1009 | 1014 | 1015 | 1016
        | 1021 | 1022 | 1028 | 1040 | 1115 | 1182 | 1183 | 1185 | 1187 | 1231 | 2951 | 3221
        | 3807 => PgTypeTag::Array(oid),
        _ => PgTypeTag::Other(oid),
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_pg_value<Ext>(
    value: PgValue<'_>,
    field_name: Option<&str>,
    origin: Option<DynamicColumnOrigin<'_>>,
    declared_sql_type: Option<diesel::sql_types::SqlTypeDescriptor>,
    tag: PgTypeTag,
    pg_array_subscripts: Option<&[i32]>,
    extension: &Ext,
    allow_extension: bool,
) -> deserialize::Result<BackendDynamicValue<Pg, Ext::Value>>
where
    Ext: DynamicValueExtension<Pg>,
{
    let context = DynamicDecodeContext::from_parts(
        field_name,
        origin,
        declared_sql_type,
        tag,
        pg_array_subscripts,
    );

    if allow_extension && extension.claims(&context) {
        return extension.decode(&context, value).map(DynamicValue::Custom);
    }

    if let Some(declared) = declared_sql_type {
        if let Some(element) = declared.element() {
            return decode_array(value, field_name, origin, Some(element), extension)
                .map(PgBackendValue::Array)
                .map(DynamicValue::Backend);
        }
        if declared.is_sql_type::<st::Bool>() {
            return read::<st::Bool, bool>(value).map(DynamicValue::Bool);
        }
        if declared.is_sql_type::<st::SmallInt>() {
            return read::<st::SmallInt, i16>(value)
                .map(i64::from)
                .map(DynamicValue::Integer);
        }
        if declared.is_sql_type::<st::Integer>() {
            return read::<st::Integer, i32>(value)
                .map(i64::from)
                .map(DynamicValue::Integer);
        }
        if declared.is_sql_type::<st::BigInt>() {
            return read::<st::BigInt, i64>(value).map(DynamicValue::Integer);
        }
        if declared.is_sql_type::<st::Float>() {
            return read::<st::Float, f32>(value)
                .map(f64::from)
                .map(DynamicValue::Float);
        }
        if declared.is_sql_type::<st::Double>() {
            return read::<st::Double, f64>(value).map(DynamicValue::Float);
        }
        if declared.is_sql_type::<st::Text>() {
            return read::<st::Text, String>(value).map(DynamicValue::Text);
        }
        if declared.is_sql_type::<st::Binary>() {
            return read::<st::Binary, Vec<u8>>(value).map(DynamicValue::Bytes);
        }
        if let Some(backend) = decode_declared_backend(declared, value)? {
            return Ok(DynamicValue::Backend(backend));
        }
    }

    match tag {
        PgTypeTag::Scalar(oid) => decode_scalar(oid, value),
        PgTypeTag::Array(_) => decode_array(value, field_name, origin, None, extension)
            .map(PgBackendValue::Array)
            .map(DynamicValue::Backend),
        PgTypeTag::Other(oid) => Ok(DynamicValue::Backend(PgBackendValue::Opaque {
            oid,
            bytes: value.as_bytes().to_owned(),
        })),
    }
}

fn decode_declared_backend<E>(
    declared: diesel::sql_types::SqlTypeDescriptor,
    value: PgValue<'_>,
) -> deserialize::Result<Option<PgBackendValue<E>>> {
    if declared.is_sql_type::<st::Date>() {
        return read::<st::Date, PgDate>(value)
            .map(PgBackendValue::Date)
            .map(Some);
    }
    if declared.is_sql_type::<st::Time>() {
        return read::<st::Time, PgTime>(value)
            .map(PgBackendValue::Time)
            .map(Some);
    }
    if declared.is_sql_type::<st::Timestamp>() {
        return read::<st::Timestamp, PgTimestamp>(value)
            .map(PgBackendValue::Timestamp)
            .map(Some);
    }
    if declared.is_sql_type::<st::Timestamptz>() {
        return read::<st::Timestamptz, PgTimestamp>(value)
            .map(PgBackendValue::Timestamptz)
            .map(Some);
    }
    if declared.is_sql_type::<st::Interval>() {
        return read::<st::Interval, PgInterval>(value)
            .map(PgBackendValue::Interval)
            .map(Some);
    }
    if declared.is_sql_type::<st::Numeric>() {
        return read::<st::Numeric, PgNumeric>(value)
            .map(PgBackendValue::Numeric)
            .map(Some);
    }
    if declared.is_sql_type::<st::Money>() {
        return read::<st::Money, PgMoney>(value)
            .map(PgBackendValue::Money)
            .map(Some);
    }
    if declared.is_sql_type::<st::Uuid>() {
        return read::<st::Uuid, PgRawUuid>(value)
            .map(|value| PgBackendValue::Uuid(value.0))
            .map(Some);
    }
    if declared.is_sql_type::<st::Json>() {
        return read::<st::Json, PgRawJson>(value)
            .map(|value| PgBackendValue::Json(value.0))
            .map(Some);
    }
    if declared.is_sql_type::<st::Jsonb>() {
        return read::<st::Jsonb, PgRawJson>(value)
            .map(|value| PgBackendValue::Jsonb(value.0))
            .map(Some);
    }
    if declared.is_sql_type::<st::MacAddr>() {
        return read::<st::MacAddr, [u8; 6]>(value)
            .map(PgBackendValue::MacAddr)
            .map(Some);
    }
    if declared.is_sql_type::<st::MacAddr8>() {
        return read::<st::MacAddr8, [u8; 8]>(value)
            .map(PgBackendValue::MacAddr8)
            .map(Some);
    }
    if declared.is_sql_type::<st::Oid>() {
        return read::<st::Oid, u32>(value)
            .map(PgBackendValue::Oid)
            .map(Some);
    }
    if declared.is_sql_type::<st::CChar>() {
        return read::<st::CChar, u8>(value)
            .map(PgBackendValue::CChar)
            .map(Some);
    }
    if declared.is_sql_type::<st::PgLsn>() {
        return read::<st::PgLsn, PgLsn>(value)
            .map(PgBackendValue::PgLsn)
            .map(Some);
    }
    Ok(None)
}

fn decode_scalar<E>(
    oid: NonZeroU32,
    value: PgValue<'_>,
) -> deserialize::Result<BackendDynamicValue<Pg, E>> {
    Ok(match oid.get() {
        BOOL => DynamicValue::Bool(read::<st::Bool, bool>(value)?),
        INT2 => DynamicValue::Integer(read::<st::SmallInt, i16>(value)?.into()),
        INT4 => DynamicValue::Integer(read::<st::Integer, i32>(value)?.into()),
        INT8 => DynamicValue::Integer(read::<st::BigInt, i64>(value)?),
        FLOAT4 => DynamicValue::Float(read::<st::Float, f32>(value)?.into()),
        FLOAT8 => DynamicValue::Float(read::<st::Double, f64>(value)?),
        TEXT | VARCHAR | BPCHAR | NAME => DynamicValue::Text(read::<st::Text, String>(value)?),
        BYTEA => DynamicValue::Bytes(read::<st::Binary, Vec<u8>>(value)?),
        DATE => DynamicValue::Backend(PgBackendValue::Date(read::<st::Date, PgDate>(value)?)),
        TIME => DynamicValue::Backend(PgBackendValue::Time(read::<st::Time, PgTime>(value)?)),
        TIMESTAMP => DynamicValue::Backend(PgBackendValue::Timestamp(read::<
            st::Timestamp,
            PgTimestamp,
        >(value)?)),
        TIMESTAMPTZ => DynamicValue::Backend(PgBackendValue::Timestamptz(read::<
            st::Timestamptz,
            PgTimestamp,
        >(value)?)),
        INTERVAL => DynamicValue::Backend(PgBackendValue::Interval(read::<
            st::Interval,
            PgInterval,
        >(value)?)),
        NUMERIC => DynamicValue::Backend(PgBackendValue::Numeric(read::<st::Numeric, PgNumeric>(
            value,
        )?)),
        MONEY => DynamicValue::Backend(PgBackendValue::Money(read::<st::Money, PgMoney>(value)?)),
        UUID => DynamicValue::Backend(PgBackendValue::Uuid(read::<st::Uuid, PgRawUuid>(value)?.0)),
        JSON => DynamicValue::Backend(PgBackendValue::Json(read::<st::Json, PgRawJson>(value)?.0)),
        JSONB => DynamicValue::Backend(PgBackendValue::Jsonb(
            read::<st::Jsonb, PgRawJson>(value)?.0,
        )),
        MACADDR => DynamicValue::Backend(PgBackendValue::MacAddr(read::<st::MacAddr, [u8; 6]>(
            value,
        )?)),
        MACADDR8 => DynamicValue::Backend(PgBackendValue::MacAddr8(read::<st::MacAddr8, [u8; 8]>(
            value,
        )?)),
        OID => DynamicValue::Backend(PgBackendValue::Oid(read::<st::Oid, u32>(value)?)),
        CCHAR => DynamicValue::Backend(PgBackendValue::CChar(read::<st::CChar, u8>(value)?)),
        PGLSN => DynamicValue::Backend(PgBackendValue::PgLsn(read::<st::PgLsn, PgLsn>(value)?)),
        _ => DynamicValue::Backend(PgBackendValue::Opaque {
            oid,
            bytes: value.as_bytes().to_owned(),
        }),
    })
}

fn decode_array<Ext>(
    value: PgValue<'_>,
    field_name: Option<&str>,
    origin: Option<DynamicColumnOrigin<'_>>,
    element_descriptor: Option<diesel::sql_types::SqlTypeDescriptor>,
    extension: &Ext,
) -> deserialize::Result<PgDynamicArray<Ext::Value>>
where
    Ext: DynamicValueExtension<Pg>,
{
    let mut bytes = value.as_bytes();
    let dimensions_len = read_i32(&mut bytes)?;
    let _has_null = read_i32(&mut bytes)? != 0;
    let element_oid = NonZeroU32::new(read_u32(&mut bytes)?).unwrap_or_else(|| value.get_oid());

    let mut dimensions = Vec::new();
    for _ in 0..dimensions_len {
        dimensions.push(PgArrayDimension {
            length: read_i32(&mut bytes)?,
            lower_bound: read_i32(&mut bytes)?,
        });
    }

    let value_count = dimensions.iter().try_fold(1usize, |count, dimension| {
        usize::try_from(dimension.length)
            .ok()
            .and_then(|length| count.checked_mul(length))
    });
    let Some(value_count) = value_count else {
        return Err("invalid PostgreSQL array dimensions".into());
    };

    let mut values = Vec::with_capacity(value_count);
    let mut subscripts = dimensions
        .iter()
        .map(|dimension| dimension.lower_bound)
        .collect::<Vec<_>>();

    for idx in 0..value_count {
        let item_len = read_i32(&mut bytes)?;
        let item = if item_len < 0 {
            DynamicValue::Null
        } else {
            let item_len = usize::try_from(item_len)?;
            if bytes.len() < item_len {
                return Err("PostgreSQL array element is truncated".into());
            }
            let (item_bytes, rest) = bytes.split_at(item_len);
            bytes = rest;
            let element_value = PgValue::new_dynamic(item_bytes, &element_oid);
            decode_pg_value(
                element_value,
                field_name,
                origin,
                element_descriptor,
                type_tag(element_oid),
                Some(&subscripts),
                extension,
                true,
            )?
        };
        values.push(item);
        if idx + 1 != value_count {
            advance_subscripts(&dimensions, &mut subscripts)?;
        }
    }

    Ok(PgDynamicArray { dimensions, values })
}

fn advance_subscripts(
    dimensions: &[PgArrayDimension],
    subscripts: &mut [i32],
) -> deserialize::Result<()> {
    for idx in (0..dimensions.len()).rev() {
        let next = subscripts[idx]
            .checked_add(1)
            .ok_or("PostgreSQL array subscript overflow")?;
        let upper = dimensions[idx]
            .lower_bound
            .checked_add(dimensions[idx].length)
            .ok_or("PostgreSQL array bound overflow")?;
        if next < upper {
            subscripts[idx] = next;
            return Ok(());
        }
        subscripts[idx] = dimensions[idx].lower_bound;
    }
    Ok(())
}

fn read_i32(bytes: &mut &[u8]) -> deserialize::Result<i32> {
    if bytes.len() < 4 {
        return Err("PostgreSQL array header is truncated".into());
    }
    let (head, tail) = bytes.split_at(4);
    *bytes = tail;
    Ok(i32::from_be_bytes(head.try_into()?))
}

fn read_u32(bytes: &mut &[u8]) -> deserialize::Result<u32> {
    if bytes.len() < 4 {
        return Err("PostgreSQL array header is truncated".into());
    }
    let (head, tail) = bytes.split_at(4);
    *bytes = tail;
    Ok(u32::from_be_bytes(head.try_into()?))
}

fn read<ST, T: FromSql<ST, Pg>>(value: PgValue<'_>) -> deserialize::Result<T> {
    T::from_sql(value)
}

/// A `UUID` as PostgreSQL bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgRawUuid(pub [u8; 16]);

impl FromSql<st::Uuid, Pg> for PgRawUuid {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(Self(value.as_bytes().try_into()?))
    }
}

/// A `JSON` or `JSONB` document as text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PgRawJson(pub String);

impl FromSql<st::Json, Pg> for PgRawJson {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(Self(String::from_utf8(value.as_bytes().to_vec())?))
    }
}

impl FromSql<st::Jsonb, Pg> for PgRawJson {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let bytes = value.as_bytes();
        match bytes.split_first() {
            Some((1, rest)) => Ok(Self(String::from_utf8(rest.to_vec())?)),
            Some(_) => Err("unsupported JSONB encoding version".into()),
            None => Err("empty JSONB value".into()),
        }
    }
}
