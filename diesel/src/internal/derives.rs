#[doc(hidden)]
pub mod insertable {
    #[doc(hidden)]
    pub use crate::query_builder::insert_statement::UndecoratedInsertRecord;
}

#[doc(hidden)]
pub mod as_expression {
    #[doc(hidden)]
    pub use crate::expression::bound::Bound;
}

#[doc(hidden)]
pub mod numeric_ops {
    #[doc(hidden)]
    pub use crate::expression::ops::numeric::*;
}

#[doc(hidden)]
pub mod multiconnection {
    #[doc(hidden)]
    pub use crate::connection::private::{ConnectionSealed, MultiConnectionHelper};
    #[doc(hidden)]
    pub use crate::expression::operators::Concat;
    #[doc(hidden)]
    pub use crate::query_builder::ast_pass::AstPassHelper;
    #[doc(hidden)]
    pub use crate::query_builder::insert_statement::DefaultValues;
    #[doc(hidden)]
    pub use crate::query_builder::limit_offset_clause::{
        BoxedLimitOffsetClause, LimitOffsetClause,
    };
    #[doc(hidden)]
    pub use crate::query_builder::returning::ReturningClause;
    #[doc(hidden)]
    pub use crate::query_builder::select_statement::SelectStatement;
    #[doc(hidden)]
    pub use crate::query_builder::select_statement::boxed::BoxedSelectStatement;
    #[doc(hidden)]
    pub use crate::row::private::RowSealed;
    #[doc(hidden)]
    pub mod sql_dialect {
        #[doc(hidden)]
        pub use crate::backend::sql_dialect::*;
    }
    #[doc(hidden)]
    pub use crate::backend::private::{DieselReserveSpecialization, TrustedBackend};
    #[doc(hidden)]
    pub mod array_comparison {
        #[doc(hidden)]
        pub use crate::expression::array_comparison::*;
    }
    #[doc(hidden)]
    pub use crate::expression::exists::Exists;
    #[doc(hidden)]
    pub use crate::query_builder::from_clause::NoFromClause;
    #[doc(hidden)]
    pub use crate::query_builder::insert_statement::batch_insert::BatchInsert;
    #[doc(hidden)]
    pub use crate::row::private::PartialRow;

    #[doc(hidden)]
    pub use crate::query_builder::limit_clause::{LimitClause, NoLimitClause};
    #[doc(hidden)]
    pub use crate::query_builder::offset_clause::{NoOffsetClause, OffsetClause};

    #[doc(hidden)]
    pub use crate::query_builder::select_statement::boxed::BoxedQueryHelper;

    #[doc(hidden)]
    pub use crate::query_builder::select_statement::SelectStatementAccessor;

    #[doc(hidden)]
    #[cfg(feature = "chrono")]
    pub use chrono;

    #[doc(hidden)]
    #[cfg(feature = "time")]
    pub use time;

    #[doc(hidden)]
    #[cfg(feature = "numeric")]
    pub use bigdecimal;

    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "r2d2")]
    macro_rules! expand_r2d2 {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "r2d2"))]
    macro_rules! expand_r2d2 {
        ($($tt:tt)*) => {};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "numeric")]
    macro_rules! expand_numeric {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "numeric"))]
    macro_rules! expand_numeric {
        ($($tt:tt)*) => {};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "chrono")]
    macro_rules! expand_chrono {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "chrono"))]
    macro_rules! expand_chrono {
        ($($tt:tt)*) => {};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(feature = "time")]
    macro_rules! expand_time {
        ($($tt:tt)*) => {$($tt)*};
    }

    #[doc(hidden)]
    #[macro_export]
    #[cfg(not(feature = "time"))]
    macro_rules! expand_time {
        ($($tt:tt)*) => {};
    }

    pub use {expand_chrono, expand_numeric, expand_r2d2, expand_time};

    #[doc(hidden)]
    pub use crate::types::enum_::{
        EnumMapping, EnumVariant, IntMapping, IntegerMappingHelper, StringMapping,
    };
}

#[doc(hidden)]
pub mod sql_type {
    #[doc(hidden)]
    pub use crate::types::enum_::{EnumMapping, EnumTypeMapping};
    #[doc(hidden)]
    pub use crate::{expand_mysql, expand_pg, expand_sqlite};
}

#[doc(hidden)]
pub mod has_query {
    #[doc(hidden)]
    pub use crate::{expand_mysql, expand_pg, expand_sqlite};
}

#[doc(hidden)]
pub mod enum_ {
    #[doc(hidden)]
    pub use crate::{expand_mysql, expand_pg, expand_sqlite};

    #[doc(hidden)]
    pub use crate::types::enum_::{EnumMapping, EnumVariant};
}
