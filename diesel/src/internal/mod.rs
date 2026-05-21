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
}
