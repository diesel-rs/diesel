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
    ($Source:ident -> $Target:ty) => {
        expression_impls!($Source -> $Target, unsized);

        impl $crate::expression::AsExpression<$Source> for $Target {
            type Expression = $crate::expression::bound::Bound<$Source, Self>;

            fn as_expression(self) -> Self::Expression {
                $crate::expression::bound::Bound::new(self)
            }
        }

        impl $crate::expression::AsExpression<$crate::types::Nullable<$Source>> for $Target {
            type Expression = $crate::expression::bound::Bound<$crate::types::Nullable<$Source>, Self>;

            fn as_expression(self) -> Self::Expression {
                $crate::expression::bound::Bound::new(self)
            }
        }
    };

    ($Source:ident -> $Target:ty, unsized) => {
        impl<'expr> $crate::expression::AsExpression<$Source> for &'expr $Target {
            type Expression = $crate::expression::bound::Bound<$Source, Self>;

            fn as_expression(self) -> Self::Expression {
                $crate::expression::bound::Bound::new(self)
            }
        }

        impl<'expr, 'expr2> $crate::expression::AsExpression<$Source> for &'expr2 &'expr $Target {
            type Expression = $crate::expression::bound::Bound<$Source, Self>;

            fn as_expression(self) -> Self::Expression {
                $crate::expression::bound::Bound::new(self)
            }
        }

        impl<'expr> $crate::expression::AsExpression<$crate::types::Nullable<$Source>> for &'expr $Target {
            type Expression = $crate::expression::bound::Bound<$crate::types::Nullable<$Source>, Self>;

            fn as_expression(self) -> Self::Expression {
                $crate::expression::bound::Bound::new(self)
            }
        }

        impl<'expr, 'expr2> $crate::expression::AsExpression<$crate::types::Nullable<$Source>> for &'expr2 &'expr $Target {
            type Expression = $crate::expression::bound::Bound<$crate::types::Nullable<$Source>, Self>;

            fn as_expression(self) -> Self::Expression {
                $crate::expression::bound::Bound::new(self)
            }
        }

        impl<DB> $crate::types::ToSql<$crate::types::Nullable<$Source>, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<$Source>,
            $Target: $crate::types::ToSql<$Source, DB>,
        {
            fn to_sql<W: ::std::io::Write>(&self, out: &mut $crate::types::ToSqlOutput<W, DB>) -> Result<$crate::types::IsNull, Box<::std::error::Error+Send+Sync>> {
                $crate::types::ToSql::<$Source, DB>::to_sql(self, out)
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! queryable_impls {
    ($Source:ident -> $Target:ty) => {
        impl<DB> $crate::types::FromSqlRow<$Source, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<$Source>,
            $Target: $crate::types::FromSql<$Source, DB>,
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> Result<Self, Box<::std::error::Error+Send+Sync>> {
                $crate::types::FromSql::<$Source, DB>::from_sql(row.take())
            }
        }

        impl<DB> $crate::query_source::Queryable<$Source, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<$Source>,
            $Target: $crate::types::FromSqlRow<$Source, DB>,
        {
            type Row = Self;

            fn build(row: Self::Row) -> Self {
                row
            }
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
    };

    ($Source:ident -> ($Target:ty, $($rest:tt)+)) => {
        primitive_impls!($Source -> $Target);
        primitive_impls!($Source -> ($($rest)+));
    };

    ($Source:ident -> $Target:ty) => {
        primitive_impls!($Source);
        queryable_impls!($Source -> $Target);
        expression_impls!($Source -> $Target);
    };

    ($Source:ident) => {
        impl $crate::query_builder::QueryId for $Source {
            type QueryId = Self;

            const HAS_STATIC_QUERY_ID: bool = true;
        }

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
