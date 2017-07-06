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
macro_rules! expression_impls {
    ($($Source:ident -> $Target:ty),+,) => {
        $(
            impl<'a> $crate::expression::AsExpression<$crate::types::$Source> for $Target {
                type Expression = $crate::expression::bound::Bound<$crate::types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a, 'expr> $crate::expression::AsExpression<$crate::types::$Source> for &'expr $Target {
                type Expression = $crate::expression::bound::Bound<$crate::types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a> $crate::expression::AsExpression<$crate::types::Nullable<$crate::types::$Source>> for $Target {
                type Expression = $crate::expression::bound::Bound<$crate::types::Nullable<$crate::types::$Source>, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a, 'expr> $crate::expression::AsExpression<$crate::types::Nullable<$crate::types::$Source>> for &'expr $Target {
                type Expression = $crate::expression::bound::Bound<$crate::types::Nullable<$crate::types::$Source>, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a, DB> $crate::types::ToSql<$crate::types::Nullable<$crate::types::$Source>, DB> for $Target where
                DB: $crate::backend::Backend + $crate::types::HasSqlType<$crate::types::$Source>,
                $Target: $crate::types::ToSql<$crate::types::$Source, DB>,
            {
                fn to_sql<W: ::std::io::Write>(&self, out: &mut W) -> Result<$crate::types::IsNull, Box<::std::error::Error+Send+Sync>> {
                    $crate::types::ToSql::<$crate::types::$Source, DB>::to_sql(self, out)
                }
            }
        )+
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! queryable_impls {
    ($($Source:ident -> $Target:ty),+,) => {$(
        impl<DB> $crate::types::FromSqlRow<$crate::types::$Source, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<$crate::types::$Source>,
            $Target: $crate::types::FromSql<$crate::types::$Source, DB>,
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> Result<Self, Box<::std::error::Error+Send+Sync>> {
                $crate::types::FromSql::<$crate::types::$Source, DB>::from_sql(row.take())
            }
        }

        #[cfg(not(feature = "unstable"))]
        impl<DB> $crate::types::FromSqlRow<$crate::types::Nullable<$crate::types::$Source>, DB> for Option<$Target> where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<$crate::types::$Source>,
            Option<$Target>: $crate::types::FromSql<$crate::types::Nullable<$crate::types::$Source>, DB>,
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> Result<Self, Box<::std::error::Error+Send+Sync>> {
                $crate::types::FromSql::<$crate::types::Nullable<$crate::types::$Source>, DB>::from_sql(row.take())
            }
        }

        impl<DB> $crate::query_source::Queryable<$crate::types::$Source, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<$crate::types::$Source>,
            $Target: $crate::types::FromSqlRow<$crate::types::$Source, DB>,
        {
            type Row = Self;

            fn build(row: Self::Row) -> Self {
                row
            }
        }
    )+}
}

#[doc(hidden)]
#[macro_export]
macro_rules! primitive_impls {
    ($Source:ident -> (, $($rest:tt)*)) => {
        primitive_impls!($Source -> ($($rest)*));
    };

    ($Source:ident -> (sqlite: ($tpe:ident) $($rest:tt)*)) => {
        #[cfg(feature = "sqlite")]
        impl $crate::types::HasSqlType<$crate::types::$Source> for $crate::sqlite::Sqlite {
            fn metadata() -> $crate::sqlite::SqliteType {
                $crate::sqlite::SqliteType::$tpe
            }
        }

        primitive_impls!($Source -> ($($rest)*));
    };

    ($Source:ident -> (pg: ($oid:expr, $array_oid:expr) $($rest:tt)*)) => {
        #[cfg(feature = "postgres")]
        impl $crate::types::HasSqlType<$crate::types::$Source> for $crate::pg::Pg {
            fn metadata() -> $crate::pg::PgTypeMetadata {
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
        impl $crate::types::HasSqlType<$crate::types::$Source> for $crate::mysql::Mysql {
            fn metadata() -> $crate::mysql::MysqlType {
                $crate::mysql::MysqlType::$tpe
            }
        }

        primitive_impls!($Source -> ($($rest)*));
    };

    // Done implementing type metadata, no body
    ($Source:ident -> ()) => {
    };

    ($Source:ident -> ($Target:ty, $($rest:tt)+)) => {
        primitive_impls!($Source -> $Target);
        primitive_impls!($Source -> ($($rest)+));
    };

    ($Source:ident -> $Target:ty) => {
        primitive_impls!($Source);
        queryable_impls!($Source -> $Target,);
        expression_impls!($Source -> $Target,);
    };

    ($Source:ident) => {
        impl $crate::types::HasSqlType<$crate::types::$Source> for $crate::backend::Debug {
            fn metadata() {}
        }

        impl $crate::query_builder::QueryId for $crate::types::$Source {
            type QueryId = Self;

            fn has_static_query_id() -> bool {
                true
            }
        }

        impl $crate::types::NotNull for $crate::types::$Source {
        }

        impl $crate::types::SingleValue for $crate::types::$Source {
        }
    }
}

mod date_and_time;
mod debug;
pub mod floats;
mod integers;
pub mod option;
mod primitives;
mod tuples;
mod decimal;
