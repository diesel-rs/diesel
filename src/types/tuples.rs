use super::{NativeSqlType, FromSql};
use {Queriable, Table, Column, QuerySource};
use query_source::SelectableColumn;
use row::Row;

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

            impl<$($T),+,$($ST),+> FromSql<($($ST),+)> for ($($T),+) where
                $($T: FromSql<$ST>),+,
                $($ST: NativeSqlType),+
            {
                #[allow(unused_assignments)]
                fn from_sql<T: Row>(row: &mut T) -> Self {
                    ($($T::from_sql(row)),+)
                }
            }

            impl<$($T),+,$($ST),+> Queriable<($($ST),+)> for ($($T),+) where
                $($T: FromSql<$ST>),+,
                $($ST: NativeSqlType),+
            {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }

            impl<$($T),+, $($ST),+, $($TT),+>
                Column<($($ST),+), ($($TT),+)> for ($($T),+) where
                $($T: Column<$ST, $TT>),+,
            {
                #[allow(non_snake_case)]
                fn name(&self) -> String {
                    let parts: &[String] = e!(&[$(self.$idx.name()),*]);
                    parts.join(", ")
                }
            }

            impl<$($T),+, $($ST),+, $($TT),+, QS>
                SelectableColumn<($($ST),+), ($($TT),+), QS>
                for ($($T),+) where
                $($T: SelectableColumn<$ST, $TT, QS>),+,
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
