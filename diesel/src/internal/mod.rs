//! This module contains API definitions which are not considered
//! to be part diesels public API
//!
//! **DO NOT EXPECT ANY STABILITY GUARANTEES HERE**

pub mod alias_macro;
pub mod derives;
pub mod migrations;
pub mod operators_macro;
pub mod sql_functions;
pub mod table_macro;

/// Used by `declare_sql_function` for functions declared with `#[query_source]`
pub mod table_functions {
    pub use crate::query_source::function_call::{
        BaseTypeColumnAlias, NoColumnAliases, TableFunction,
    };
}

mod helper_macros {
    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "postgres_backend")]
    macro_rules! expand_pg {
        ($($tt:tt)*) => {$($tt)*};
    }
    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "postgres_backend"))]
    macro_rules! expand_pg {
        ($($tt:tt)*) => {};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "mysql_backend")]
    macro_rules! expand_mysql {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "mysql_backend"))]
    macro_rules! expand_mysql {
        ($($tt:tt)*) => {};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "mariadb_backend")]
    macro_rules! expand_mariadb {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "mariadb_backend"))]
    macro_rules! expand_mariadb {
        ($($tt:tt)*) => {};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "__sqlite-shared")]
    macro_rules! expand_sqlite {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "__sqlite-shared"))]
    macro_rules! expand_sqlite {
        ($($tt:tt)*) => {};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(any(feature = "mysql_backend", feature = "mariadb_backend"))]
    macro_rules! expand_mysql_like {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(any(feature = "mysql_backend", feature = "mariadb_backend")))]
    macro_rules! expand_mysql_like {
        ($($tt:tt)*) => {};
    }
}
