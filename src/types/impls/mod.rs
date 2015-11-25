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

            impl<'a> ToSql<types::Nullable<types::$Source>> for $Target {
                fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
                    <Self as ToSql<types::$Source>>::to_sql(self, out)
                }
            }
        )+
    }
}

macro_rules! primitive_impls {
    ($($Source:ident -> ($Target:ty, $oid:expr)),+,) => {
        $(
            impl NativeSqlType for types::$Source {
                fn oid(&self) -> u32 {
                    $oid
                }

                fn new() -> Self {
                    types::$Source
                }
            }

            impl Queriable<types::$Source> for $Target {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }
        )+
        expression_impls!($($Source -> $Target),+,);
    }
}

mod array;
pub mod date_and_time;
mod floats;
mod integers;
mod option;
mod primitives;
mod tuples;
