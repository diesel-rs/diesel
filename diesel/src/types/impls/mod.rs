/// Gets the value out of an option, or returns an error.
///
/// This is used by `FromSql` implementations.
#[macro_export]
macro_rules! not_none {
    ($bytes:expr) => {
        match $bytes {
            Some(bytes) => bytes,
            None => return Err(Box::new($crate::types::impls::option::UnexpectedNullError {
                msg: "Unexpected null for non-null column".to_string(),
            })),
        }
    }
}

mod date_and_time;
pub mod floats;
mod integers;
pub mod option;
mod primitives;
mod tuples;
mod decimal;
