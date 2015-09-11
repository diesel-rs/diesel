macro_rules! not_none {
    ($bytes:expr) => {
        match $bytes {
            Some(bytes) => bytes,
            None => return Err(Box::new(UnexpectedNullError {
                msg: "Unexpected null for non-null column".to_string(),
            })),
        }
    }
}

mod primitives;
mod tuples;
mod array;
mod option;
