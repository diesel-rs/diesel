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

macro_rules! primitive_impls {
    ($($Source:ident -> ($Target:ty, $oid:expr)),+,) => {
        $(
            impl NativeSqlType for types::$Source {
                fn oid() -> u32 {
                    $oid
                }
            }

            impl Queriable<types::$Source> for $Target {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }

            impl AsExpression<types::$Source> for $Target {
                type Expression = Bound<types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'a> AsExpression<types::$Source> for &'a $Target {
                type Expression = Bound<types::$Source, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl AsExpression<types::Nullable<types::$Source>> for $Target {
                type Expression = <Self as AsExpression<types::$Source>>::Expression;

                fn as_expression(self) -> Self::Expression {
                    AsExpression::<types::$Source>::as_expression(self)
                }
            }

            impl<'a> AsExpression<types::Nullable<types::$Source>> for &'a $Target {
                type Expression = <Self as AsExpression<types::$Source>>::Expression;

                fn as_expression(self) -> Self::Expression {
                    AsExpression::<types::$Source>::as_expression(self)
                }
            }
        )+
    }
}

mod array;
pub mod date_and_time;
mod floats;
mod integers;
mod option;
mod primitives;
mod tuples;
