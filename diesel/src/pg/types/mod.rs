mod array;
pub mod date_and_time;
pub mod floats;
mod integers;
mod primitives;
#[cfg(feature = "uuid")]
mod uuid;

#[doc(hidden)]
pub mod sql_types {
    #[derive(Debug, Clone, Copy, Default)] pub struct Oid;
    #[derive(Debug, Clone, Copy, Default)] pub struct Array<T>(T);
    pub type SmallSerial = ::types::SmallInt;
    pub type Serial = ::types::Integer;
    pub type BigSerial = ::types::BigInt;
    #[cfg(feature = "uuid")]
    #[derive(Debug, Clone, Copy, Default)] pub struct Uuid;
    pub type Bytea = ::types::Binary;
}
