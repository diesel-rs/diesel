//! PostgreSQL arrays diesel writes and reads, checked against a model of the binary format.
//!
//! The model follows PostgreSQL's `array_send`: the dimension count, a null flag set exactly when
//! an element is NULL, and the element type's OID; a length and lower bound per dimension; then
//! each element prefixed by its length, which is `-1` for NULL.

use std::fmt::Debug;
use std::num::NonZeroU32;

use arbitrary::Arbitrary;
use diesel::deserialize::FromSql;
use diesel::pg::data_types::NdArray;
use diesel::pg::{Pg, PgMetadataLookup, PgTypeMetadata, PgValue};
use diesel::query_builder::BindCollector;
use diesel::query_builder::bind_collector::RawBytesBindCollector;
use diesel::serialize::ToSql;
use diesel::sql_types::{Array, HasSqlType, Integer, Nullable, Text};

/// PostgreSQL's `MAXDIM`.
const MAX_DIMENSIONS: usize = 6;
const MAX_ELEMENTS: usize = 64;

#[derive(Arbitrary, Debug)]
pub enum ArrayInput {
    Int4(ArrayCase<i32>),
    Text(ArrayCase<String>),
}

#[derive(Arbitrary, Debug)]
pub struct ArrayCase<T> {
    /// Elements in storage order, `None` being NULL; only the first 64 are used.
    pub elements: Vec<Option<T>>,
    /// Dimension lengths for the multi-dimensional decode, each taken modulo 5.
    pub dimensions: Vec<u8>,
    /// Lower bound of every dimension in the multi-dimensional decode, which diesel ignores.
    pub lower_bound: i32,
}

/// An element type whose OIDs and binary encoding are written here, independently of diesel.
trait ModelElement: Clone + Debug + PartialEq {
    type SqlType: 'static;
    const OID: u32;
    const ARRAY_OID: u32;

    fn encode(&self) -> Vec<u8>;
}

impl ModelElement for i32 {
    type SqlType = Integer;
    const OID: u32 = 23;
    const ARRAY_OID: u32 = 1007;

    fn encode(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl ModelElement for String {
    type SqlType = Text;
    const OID: u32 = 25;
    const ARRAY_OID: u32 = 1009;

    fn encode(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

/// Built-in element types have static OIDs, so encoding them never looks one up.
struct StaticOidsOnly;

impl PgMetadataLookup for StaticOidsOnly {
    fn lookup_type(&mut self, type_name: &str, _schema: Option<&str>) -> PgTypeMetadata {
        unreachable!("built-in type `{type_name}` needs no lookup")
    }
}

/// Encode and decode one array, returning the first disagreement with the model.
pub fn run_case(input: &ArrayInput) -> Result<(), String> {
    match input {
        ArrayInput::Int4(case) => check(case),
        ArrayInput::Text(case) => check(case),
    }
}

fn check<T>(case: &ArrayCase<T>) -> Result<(), String>
where
    T: ModelElement,
    [Option<T>]: ToSql<Array<Nullable<T::SqlType>>, Pg>,
    Vec<Option<T>>: FromSql<Array<Nullable<T::SqlType>>, Pg>,
    NdArray<Option<T>>: FromSql<Array<Nullable<T::SqlType>>, Pg>,
    Pg: HasSqlType<Array<Nullable<T::SqlType>>>,
{
    let elements = &case.elements[..case.elements.len().min(MAX_ELEMENTS)];

    // diesel writes one dimension starting at index 1.
    let encoded = encode(elements)?;
    let expected = model_encode(&[elements.len()], 1, elements);
    if encoded != expected {
        return Err(format!(
            "diesel encoded {elements:?} as {encoded:?}, expected {expected:?}"
        ));
    }
    expect_decoded(&encoded, &[elements.len()], elements)?;

    let dims = dimensions(&case.dimensions);
    let count = if dims.is_empty() {
        0
    } else {
        dims.iter().product()
    };
    let data = elements
        .iter()
        .cloned()
        .chain(std::iter::repeat(None))
        .take(count)
        .collect::<Vec<_>>();
    expect_decoded(&model_encode(&dims, case.lower_bound, &data), &dims, &data)
}

/// Serialize `elements` the way diesel serializes a bind parameter.
fn encode<T>(elements: &[Option<T>]) -> Result<Vec<u8>, String>
where
    T: ModelElement,
    [Option<T>]: ToSql<Array<Nullable<T::SqlType>>, Pg>,
    Pg: HasSqlType<Array<Nullable<T::SqlType>>>,
{
    let mut collector = RawBytesBindCollector::<Pg>::new();
    collector
        .push_bound_value::<Array<Nullable<T::SqlType>>, _>(
            elements,
            &mut StaticOidsOnly as &mut dyn PgMetadataLookup,
        )
        .map_err(|e| format!("diesel failed to encode {elements:?}: {e}"))?;

    let array_oid = collector.metadata[0].oid().map_err(|e| e.to_string())?;
    if array_oid != T::ARRAY_OID {
        return Err(format!(
            "diesel typed {elements:?} with OID {array_oid}, expected {}",
            T::ARRAY_OID
        ));
    }
    collector
        .binds
        .pop()
        .flatten()
        .ok_or_else(|| format!("diesel encoded {elements:?} as NULL"))
}

/// Decode `bytes` with both of diesel's array types and compare with the model's array.
fn expect_decoded<T>(bytes: &[u8], dims: &[usize], data: &[Option<T>]) -> Result<(), String>
where
    T: ModelElement,
    Vec<Option<T>>: FromSql<Array<Nullable<T::SqlType>>, Pg>,
    NdArray<Option<T>>: FromSql<Array<Nullable<T::SqlType>>, Pg>,
{
    let array_oid = NonZeroU32::new(T::ARRAY_OID).expect("array OIDs are nonzero");

    let array = <NdArray<Option<T>> as FromSql<Array<Nullable<T::SqlType>>, Pg>>::from_sql(
        PgValue::new(bytes, &array_oid),
    )
    .map_err(|e| format!("diesel failed to decode the {dims:?} array {data:?}: {e}"))?;
    if array.dims != dims || array.data != data {
        return Err(format!(
            "diesel decoded the {dims:?} array {data:?} as {:?} {:?}",
            array.dims, array.data
        ));
    }

    let vec = <Vec<Option<T>> as FromSql<Array<Nullable<T::SqlType>>, Pg>>::from_sql(PgValue::new(
        bytes, &array_oid,
    ));
    match (dims.len(), vec) {
        (0 | 1, Ok(vec)) if vec == data => Ok(()),
        // `Vec` only holds one dimension.
        (2.., Err(_)) => Ok(()),
        (_, vec) => Err(format!(
            "diesel decoded the {dims:?} array {data:?} as the Vec {vec:?}"
        )),
    }
}

/// Encode an array the way PostgreSQL's `array_send` does.
fn model_encode<T: ModelElement>(
    dims: &[usize],
    lower_bound: i32,
    elements: &[Option<T>],
) -> Vec<u8> {
    fn int(value: usize) -> [u8; 4] {
        i32::try_from(value)
            .expect("bounded by the fuzz input")
            .to_be_bytes()
    }

    let mut out = Vec::new();
    out.extend(int(dims.len()));
    out.extend(i32::from(elements.iter().any(Option::is_none)).to_be_bytes());
    out.extend(T::OID.to_be_bytes());
    for &len in dims {
        out.extend(int(len));
        out.extend(lower_bound.to_be_bytes());
    }
    for element in elements {
        match element {
            None => out.extend((-1_i32).to_be_bytes()),
            Some(value) => {
                let bytes = value.encode();
                out.extend(int(bytes.len()));
                out.extend(bytes);
            }
        }
    }
    out
}

/// Up to `MAX_DIMENSIONS` lengths from 0 to 4 whose product stays within `MAX_ELEMENTS`.
fn dimensions(raw: &[u8]) -> Vec<usize> {
    let mut dims = Vec::new();
    let mut count = 1;
    for len in raw
        .iter()
        .take(MAX_DIMENSIONS)
        .map(|&len| usize::from(len % 5))
    {
        if count * len > MAX_ELEMENTS {
            break;
        }
        count *= len;
        dims.push(len);
    }
    dims
}
