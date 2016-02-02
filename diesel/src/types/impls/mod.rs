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
            impl<'a> AsExpression<types::$Source> for $Target {
                type Expression = Bound<types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'a: 'expr, 'expr> AsExpression<types::$Source> for &'expr $Target {
                type Expression = Bound<types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'a> AsExpression<types::Nullable<types::$Source>> for $Target {
                type Expression = Bound<types::Nullable<types::$Source>, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'a: 'expr, 'expr> AsExpression<types::Nullable<types::$Source>> for &'a $Target {
                type Expression = Bound<types::Nullable<types::$Source>, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'a, DB> ToSql<types::Nullable<types::$Source>, DB> for $Target where
                DB: $crate::backend::Backend + types::HasSqlType<types::$Source>,
                $Target: ToSql<types::$Source, DB>,
            {
                fn to_sql<W: ::std::io::Write>(&self, out: &mut W) -> Result<IsNull, Box<::std::error::Error>> {
                    ToSql::<types::$Source, DB>::to_sql(self, out)
                }
            }
        )+
    }
}

macro_rules! queryable_impls {
    ($($Source:ident -> $Target:ty),+,) => {$(
        //FIXME: This can be made generic w/ specialization by making FromSql imply FromSqlRow
        impl<DB> $crate::types::FromSqlRow<types::$Source, DB> for $Target where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<types::$Source>,
            $Target: $crate::types::FromSql<types::$Source, DB>,
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> Result<Self, Box<::std::error::Error>> {
                $crate::types::FromSql::<types::$Source, DB>::from_sql(row.take())
            }
        }

        //FIXME: This can be made generic w/ specialization by making FromSql imply FromSqlRow
        impl<DB> $crate::types::FromSqlRow<types::Nullable<types::$Source>, DB> for Option<$Target> where
            DB: $crate::backend::Backend + $crate::types::HasSqlType<types::$Source>,
            Option<$Target>: $crate::types::FromSql<types::Nullable<types::$Source>, DB>,
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> Result<Self, Box<::std::error::Error>> {
                $crate::types::FromSql::<types::Nullable<types::$Source>, DB>::from_sql(row.take())
            }
        }

        impl<DB> Queryable<types::$Source, DB> for $Target where
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
        #[cfg(feature = "postgres")]
        impl types::HasSqlType<types::$Source> for $crate::pg::Pg {
            fn metadata() -> $crate::pg::PgTypeMetadata {
                $crate::pg::PgTypeMetadata {
                    oid: $oid,
                    array_oid: $array_oid,
                }
            }
        }

        primitive_impls!($Source -> $Target);
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

        impl types::NotNull for types::$Source {
        }
    }
}

pub mod floats;
mod integers;
pub mod option;
mod primitives;
mod tuples;
