use super::{NativeSqlType, FromSqlRow};
use {Queriable, Table, Column, QuerySource};
use query_source::SelectableColumn;
use row::Row;
use std::error::Error;

// FIXME(https://github.com/rust-lang/rust/issues/19630) Remove this work-around
macro_rules! e {
    ($e:expr) => { $e }
}

macro_rules! tuple_impls {
    ($(
        $Tuple:ident {
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

            impl<$($T),+,$($ST),+> Queriable<($($ST),+)> for ($($T),+) where
                $($T: FromSqlRow<$ST>),+,
                $($ST: NativeSqlType),+
            {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }

            impl<$($T),+, $($ST),+, $($TT),+>
                Column<($($TT),+)> for ($($T),+) where
                $($T: Column<$TT, SqlType=$ST>),+,
                $($ST: NativeSqlType),+,
            {
                type SqlType = ($($ST),+);

                #[allow(non_snake_case)]
                fn name(&self) -> String {
                    let parts: &[String] = e!(&[$(self.$idx.name()),*]);
                    parts.join(", ")
                }
            }

            impl<$($T),+, $($TT),+, QS>
                SelectableColumn<($($TT),+), QS>
                for ($($T),+) where
                $($T: SelectableColumn<$TT, QS>),+,
                QS: QuerySource,
            {}
        )+
    }
}

tuple_impls! {
    T2 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
    }
    T3 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
    }
    T4 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
    }
    T5 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
    }
    T6 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
    }
    T7 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
    }
    T8 {
        (0) -> A, SA, TA,
        (1) -> B, SB, TB,
        (2) -> C, SC, TC,
        (3) -> D, SD, TD,
        (4) -> E, SE, TE,
        (5) -> F, SF, TF,
        (6) -> G, SG, TG,
        (7) -> H, SH, TH,
    }
    T9 {
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
    T10 {
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
    T11 {
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
    T12 {
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
