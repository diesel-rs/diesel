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

macro_rules! expression_impls {
    ($($Source:ident -> $Target:ty),+,) => {
        $(
            impl<'a> $crate::expression::AsExpression<types::$Source> for $Target {
                type Expression = $crate::expression::bound::Bound<types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a, 'expr> $crate::expression::AsExpression<types::$Source> for &'expr $Target {
                type Expression = $crate::expression::bound::Bound<types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a> $crate::expression::AsExpression<$crate::types::Nullable<types::$Source>> for $Target {
                type Expression = $crate::expression::bound::Bound<$crate::types::Nullable<types::$Source>, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a, 'expr> $crate::expression::AsExpression<$crate::types::Nullable<types::$Source>> for &'expr $Target {
                type Expression = $crate::expression::bound::Bound<$crate::types::Nullable<types::$Source>, Self>;

                fn as_expression(self) -> Self::Expression {
                    $crate::expression::bound::Bound::new(self)
                }
            }

            impl<'a, DB> $crate::types::ToSql<$crate::types::Nullable<types::$Source>, DB> for $Target where
                DB: $crate::backend::Backend + $crate::types::HasSqlType<types::$Source>,
                $Target: $crate::types::ToSql<types::$Source, DB>,
            {
                fn to_sql<W: ::std::io::Write>(&self, out: &mut W) -> Result<$crate::types::IsNull, Box<::std::error::Error+Send+Sync>> {
                    $crate::types::ToSql::<types::$Source, DB>::to_sql(self, out)
                }
            }
        )+
    }
}

macro_rules! queryable_impls {
    ($($Source:ident -> $Target:ty),+,) => {$(
        impl<DB> $crate::types::FromSqlRow<types::$Source, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<types::$Source>,
            $Target: $crate::types::FromSql<types::$Source, DB>,
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> Result<Self, Box<::std::error::Error+Send+Sync>> {
                $crate::types::FromSql::<types::$Source, DB>::from_sql(row.take())
            }
        }

        #[cfg(not(feature = "unstable"))]
        impl<DB> $crate::types::FromSqlRow<$crate::types::Nullable<types::$Source>, DB> for Option<$Target> where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<types::$Source>,
            Option<$Target>: $crate::types::FromSql<$crate::types::Nullable<types::$Source>, DB>,
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> Result<Self, Box<::std::error::Error+Send+Sync>> {
                $crate::types::FromSql::<$crate::types::Nullable<types::$Source>, DB>::from_sql(row.take())
            }
        }

        impl<DB> $crate::query_source::Queryable<types::$Source, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<types::$Source>,
            $Target: $crate::types::FromSqlRow<types::$Source, DB>,
        {
            type Row = Self;

            fn build(row: Self::Row) -> Self {
                row
            }
        }
    )+}
}

macro_rules! primitive_impls {
    ($Source:ident -> ($Target:ty, pg: ($oid:expr, $array_oid:expr), sqlite: ($tpe:ident))) => {
        #[cfg(feature = "sqlite")]
        impl types::HasSqlType<types::$Source> for $crate::sqlite::Sqlite {
            fn metadata() -> $crate::sqlite::SqliteType {
                $crate::sqlite::SqliteType::$tpe
            }
        }

        primitive_impls!($Source -> ($Target, pg: ($oid, $array_oid)));
    };

    ($Source:ident -> ($Target:ty, pg: ($oid:expr, $array_oid:expr))) => {
        primitive_impls!($Source -> (pg: ($oid, $array_oid)));
        primitive_impls!($Source -> $Target);
    };

    ($Source:ident -> (pg: ($oid:expr, $array_oid:expr))) => {
        #[cfg(feature = "postgres")]
        impl types::HasSqlType<types::$Source> for $crate::pg::Pg {
            fn metadata() -> $crate::pg::PgTypeMetadata {
                $crate::pg::PgTypeMetadata {
                    oid: $oid,
                    array_oid: $array_oid,
                }
            }
        }
    };

    ($Source:ident -> $Target:ty) => {
        primitive_impls!($Source);
        queryable_impls!($Source -> $Target,);
        expression_impls!($Source -> $Target,);
    };

    ($Source:ident) => {
        impl types::HasSqlType<types::$Source> for $crate::backend::Debug {
            fn metadata() {}
        }

        impl $crate::query_builder::QueryId for types::$Source {
            type QueryId = Self;

            fn has_static_query_id() -> bool {
                true
            }
        }

        impl types::NotNull for types::$Source {
        }
    }
}

macro_rules! debug_to_sql {
    ($sql_type:ty, $ty:ty) => {
        impl $crate::types::ToSql<$sql_type, $crate::backend::Debug> for $ty {
            fn to_sql<W: ::std::io::Write>(&self, _: &mut W) -> Result<$crate::types::IsNull, Box<::std::error::Error+Send+Sync>> {
                Ok($crate::types::IsNull::No)
            }
        }
    };
}

mod date_and_time;
pub mod floats;
mod integers;
pub mod option;
mod primitives;
mod tuples;
