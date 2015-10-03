use expression::{Expression, SelectableExpression, NonAggregate};
use persistable::{AsBindParam, InsertableColumns};
use row::Row;
use std::error::Error;
use types::{NativeSqlType, FromSqlRow, ValuesToSql, Nullable};
use {Queriable, Table, Column};

// FIXME(https://github.com/rust-lang/rust/issues/19630) Remove this work-around
macro_rules! e {
    ($e:expr) => { $e }
}

macro_rules! tuple_impls {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<$($T:NativeSqlType),+> NativeSqlType for ($($T,)+) {}

            impl<$($T),+,$($ST),+> FromSqlRow<($($ST),+)> for ($($T),+) where
                $($T: FromSqlRow<$ST>),+,
                $($ST: NativeSqlType),+
            {
                fn build_from_row<T: Row>(row: &mut T) -> Result<Self, Box<Error>> {
                    Ok(($(try!($T::build_from_row(row))),+))
                }
            }

            impl<$($T),+,$($ST),+> FromSqlRow<Nullable<($($ST),+)>> for Option<($($T),+)> where
                $($T: FromSqlRow<$ST>),+,
                $($ST: NativeSqlType),+
            {
                fn build_from_row<T: Row>(row: &mut T) -> Result<Self, Box<Error>> {
                    if e!(row.next_is_null($Tuple)) {
                        Ok(None)
                    } else {
                        Ok(Some(($(try!($T::build_from_row(row))),+)))
                    }
                }
            }

            impl<$($T),+,$($ST),+> ValuesToSql<($($ST),+)> for ($($T),+) where
                $($T: ValuesToSql<$ST>),+,
                $($ST: NativeSqlType),+
            {
                fn values_to_sql(&self) -> Result<Vec<Option<Vec<u8>>>, Box<Error>> {
                    let values = e!(vec![$(try!(self.$idx.values_to_sql())),*]);
                    Ok(values.into_iter().flat_map(|v| v).collect())
                }
            }

            impl<$($T),+,$($ST),+> Queriable<($($ST),+)> for ($($T),+) where
                $($T: Queriable<$ST>),+,
                $($ST: NativeSqlType),+
            {
                type Row = ($($T::Row),+);

                fn build(row: Self::Row) -> Self {
                    ($($T::build(e!(row.$idx))),+)
                }
            }

            impl<$($T: Expression + NonAggregate),+> Expression for ($($T),+) {
                type SqlType = ($(<$T as Expression>::SqlType),+);

                fn to_sql(&self) -> String {
                    let parts: &[String] = e!(&[$(self.$idx.to_sql()),*]);
                    parts.join(", ")
                }

                fn binds(&self) -> Vec<Option<Vec<u8>>> {
                    let mut result = Vec::new();
                    $(result.append(&mut e!(self.$idx.binds()));)+
                    result
                }
            }

            impl<$($T: Expression + NonAggregate),+> NonAggregate for ($($T),+) {
            }

            impl<$($T: Column<Table=T>),+, T: Table> InsertableColumns<T> for ($($T),+) {
                type SqlType = ($(<$T as Column>::SqlType),+);

                fn names(&self) -> String {
                    let parts: &[String] = e!(&[$(self.$idx.name()),*]);
                    parts.join(", ")
                }
            }

            impl<$($T),+, $($ST),+, QS>
                SelectableExpression<QS, ($($ST),+)>
                for ($($T),+) where
                $($ST: NativeSqlType),+,
                $($T: SelectableExpression<QS, $ST>),+,
                ($($T),+): Expression,
            {
            }

            impl<$($T),+, $($ST),+, QS>
                SelectableExpression<QS, Nullable<($($ST),+)>>
                for ($($T),+) where
                $($ST: NativeSqlType),+,
                $($T: SelectableExpression<QS, Nullable<$ST>>),+,
                ($($T),+): Expression,
            {
            }

            impl<$($T),+, $($ST),+> AsBindParam<($($ST),+)> for ($($T),+) where
                $($T: AsBindParam<$ST>),+,
                $($ST: NativeSqlType),+,
            {
                fn as_bind_param(&self, idx: &mut usize) -> String {
                    e!([$(self.$idx.as_bind_param(idx)),+].join(","))
                }

                fn as_bind_param_for_insert(&self, idx: &mut usize) -> String {
                    e!([$(self.$idx.as_bind_param_for_insert(idx)),+].join(","))
                }
            }
        )+
    }
}

tuple_impls! {
    2 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
    }
    3 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
    }
    4 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
    }
    5 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
    }
    6 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
    }
    7 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
    }
    8 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
        (7) -> H, SH, TH,
    }
    9 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
        (7) -> H, SH, TH,
        (8) -> I, SI, TI,
    }
    10 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
        (7) -> H, SH, TH,
        (8) -> I, SI, TI,
        (9) -> J, SJ, TJ,
    }
    11 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
        (7) -> H, SH, TH,
        (8) -> I, SI, TI,
        (9) -> J, SJ, TJ,
        (10) -> K, SK, TK,
    }
    12 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
        (7) -> H, SH, TH,
        (8) -> I, SI, TI,
        (9) -> J, SJ, TJ,
        (10) -> K, SK, TK,
        (11) -> L, SL, TL,
    }
}
