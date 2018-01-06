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

#[doc(hidden)]
#[macro_export]
macro_rules! primitive_impls {
    ($Source:ident -> (, $($rest:tt)*)) => {
        primitive_impls!($Source -> ($($rest)*));
    };

    ($Source:ident -> (sqlite: ($tpe:ident) $($rest:tt)*)) => {
        #[cfg(feature = "sqlite")]
        impl $crate::types::HasSqlType<$Source> for $crate::sqlite::Sqlite {
            fn metadata(_: &()) -> $crate::sqlite::SqliteType {
                $crate::sqlite::SqliteType::$tpe
            }
        }

        primitive_impls!($Source -> ($($rest)*));
    };

    ($Source:ident -> (pg: ($oid:expr, $array_oid:expr) $($rest:tt)*)) => {
        #[cfg(feature = "postgres")]
        impl $crate::types::HasSqlType<$Source> for $crate::pg::Pg {
            fn metadata(_: &$crate::pg::PgMetadataLookup) -> $crate::pg::PgTypeMetadata {
                $crate::pg::PgTypeMetadata {
                    oid: $oid,
                    array_oid: $array_oid,
                }
            }
        }

        primitive_impls!($Source -> ($($rest)*));
    };

    ($Source:ident -> (mysql: ($tpe:ident) $($rest:tt)*)) => {
        #[cfg(feature = "mysql")]
        impl $crate::types::HasSqlType<$Source> for $crate::mysql::Mysql {
            fn metadata(_: &()) -> $crate::mysql::MysqlType {
                $crate::mysql::MysqlType::$tpe
            }
        }

        primitive_impls!($Source -> ($($rest)*));
    };

    // Done implementing type metadata, no body
    ($Source:ident -> ()) => {
        primitive_impls!($Source);
    };

    ($Source:ident) => {
        impl $crate::types::NotNull for $Source {
        }

        impl $crate::types::SingleValue for $Source {
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
