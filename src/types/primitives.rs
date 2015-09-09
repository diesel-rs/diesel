extern crate byteorder;

use super::{NativeSqlType, FromSql, Nullable};
use Queriable;
use row::Row;

use self::byteorder::{ReadBytesExt, BigEndian};

macro_rules! primitive_impls {
    ($($Source:ident -> $Target:ty),+,) => {
        $(
            impl NativeSqlType for super::$Source {}

            impl Queriable<super::$Source> for $Target {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }
        )+
    }
}

primitive_impls! {
    Bool -> bool,

    SmallSerial -> i16,
    Serial -> i32,
    BigSerial -> i64,

    SmallInt -> i16,
    Integer -> i32,
    BigInt -> i64,

    Float -> f32,
    Double -> f64,

    VarChar -> String,

    Binary -> Vec<u8>,
}

impl FromSql<super::Bool> for bool {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        debug_assert!(!row.next_is_null());
        let bytes = row.take();
        bytes[0] != 0
    }
}

impl FromSql<super::SmallInt> for i16 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        debug_assert!(!row.next_is_null());
        let mut bytes = row.take();
        bytes.read_i16::<BigEndian>().unwrap()
    }
}

impl FromSql<super::SmallSerial> for i16 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        <Self as FromSql<super::SmallInt>>::from_sql(row)
    }
}

impl FromSql<super::Integer> for i32 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        debug_assert!(!row.next_is_null());
        let mut bytes = row.take();
        bytes.read_i32::<BigEndian>().unwrap()
    }
}

impl FromSql<super::Serial> for i32 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        <Self as FromSql<super::Integer>>::from_sql(row)
    }
}

impl FromSql<super::BigInt> for i64 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        debug_assert!(!row.next_is_null());
        let mut bytes = row.take();
        bytes.read_i64::<BigEndian>().unwrap()
    }
}

impl FromSql<super::BigSerial> for i64 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        <Self as FromSql<super::BigInt>>::from_sql(row)
    }
}

impl FromSql<super::Float> for f32 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        debug_assert!(!row.next_is_null());
        let mut bytes = row.take();
        bytes.read_f32::<BigEndian>().unwrap()
    }
}

impl FromSql<super::Double> for f64 {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        debug_assert!(!row.next_is_null());
        let mut bytes = row.take();
        bytes.read_f64::<BigEndian>().unwrap()
    }
}

impl FromSql<super::VarChar> for String {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        let bytes = row.take();
        String::from_utf8(bytes.into()).unwrap()
    }
}

impl FromSql<super::Binary> for Vec<u8> {
    fn from_sql<T: Row>(row: &mut T) -> Self {
        row.take().into()
    }
}

impl<T: NativeSqlType> NativeSqlType for Nullable<T> {}
impl<T, ST> FromSql<Nullable<ST>> for Option<T> where
    T: FromSql<ST>,
    ST: NativeSqlType,
{
    fn from_sql<R: Row>(row: &mut R) -> Self {
        if row.next_is_null() {
            None
        } else {
            Some(T::from_sql(row))
        }
    }
}

impl<T, ST> Queriable<Nullable<ST>> for Option<T> where
    T: FromSql<ST> + Queriable<ST>,
    ST: NativeSqlType,
{
    type Row = Self;
    fn build(row: Self) -> Self {
        row
    }
}
