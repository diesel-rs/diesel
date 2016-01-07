use expression::{Expression, SelectableExpression, NonAggregate};
use persistable::InsertableColumns;
use query_builder::{Changeset, QueryBuilder, BuildQueryResult};
use query_source::QuerySource;
use row::Row;
use std::error::Error;
use types::{NativeSqlType, FromSqlRow, ToSql, Nullable};
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
            impl<$($T:NativeSqlType),+> NativeSqlType for ($($T,)+) {
                fn oid(&self) -> u32 {
                    0
                }

                fn array_oid(&self) -> u32 {
                    0
                }

                fn new() -> Self {
                    ($($T::new()),+)
                }
            }

            impl<$($T),+,$($ST),+> FromSqlRow<($($ST),+)> for ($($T),+) where
                $($T: FromSqlRow<$ST>),+,
                $($ST: NativeSqlType),+
            {
                fn build_from_row<RowT: Row>(row: &mut RowT) -> Result<Self, Box<Error>> {
                    Ok(($(try!($T::build_from_row(row))),+))
                }
            }

            impl<$($T),+,$($ST),+> FromSqlRow<Nullable<($($ST),+)>> for Option<($($T),+)> where
                $($T: FromSqlRow<$ST>),+,
                $($ST: NativeSqlType),+
            {
                fn build_from_row<RowT: Row>(row: &mut RowT) -> Result<Self, Box<Error>> {
                    if e!(row.next_is_null($Tuple)) {
                        Ok(None)
                    } else {
                        Ok(Some(($(try!($T::build_from_row(row))),+)))
                    }
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

                fn to_sql(&self, out: &mut QueryBuilder)
                -> BuildQueryResult {
                    $(
                        if e!($idx) != 0 {
                            out.push_sql(", ");
                        }
                        try!(e!(self.$idx.to_sql(out)));
                    )+
                    Ok(())
                }

                fn to_insert_sql(&self, out: &mut QueryBuilder)
                -> BuildQueryResult {
                    $(
                        if e!($idx) != 0 {
                            out.push_sql(", ");
                        }
                        try!(e!(self.$idx.to_insert_sql(out)));
                    )+
                    Ok(())
                }
            }

            impl<$($T: Expression + NonAggregate),+> NonAggregate for ($($T),+) {
            }

            impl<$($T: Column<Table=Tab>),+, Tab: Table> InsertableColumns<Tab> for ($($T),+) {
                type SqlType = ($(<$T as Expression>::SqlType),+);

                fn names(&self) -> String {
                    let parts: &[&str] = &[$($T::name()),*];
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

            impl<Target, $($T: Changeset<Target=Target>),+> Changeset for ($($T),+) where
                Target: QuerySource,
            {
                type Target = Target;

                fn is_noop(&self) -> bool {
                    $(e!(self.$idx.is_noop()) &&)+ true
                }

                #[allow(unused_assignments)]
                fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
                    let mut needs_comma = false;
                    $(
                        let noop_element = e!(self.$idx.is_noop());
                        if !noop_element {
                            if needs_comma {
                                out.push_sql(", ");
                            }
                            try!(e!(self.$idx.to_sql(out)));
                            needs_comma = true;
                        }
                    )+
                    Ok(())
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
    13 {
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
        (12) -> M, SM, TM,
    }
    14 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
    }
    15 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
    }
    16 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
    }
}

#[cfg(feature = "large-tables")]
tuple_impls! {
    17 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
    }
    18 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
    }
    19 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
    }
    20 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
        (19) -> T, ST, TT,
    }
    21 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
        (19) -> T, ST, TT,
        (20) -> U, SU, TU,
    }
    22 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
        (19) -> T, ST, TT,
        (20) -> U, SU, TU,
        (21) -> V, SV, TV,
    }
    23 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
        (19) -> T, ST, TT,
        (20) -> U, SU, TU,
        (21) -> V, SV, TV,
        (22) -> W, SW, TW,
    }
    24 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
        (19) -> T, ST, TT,
        (20) -> U, SU, TU,
        (21) -> V, SV, TV,
        (22) -> W, SW, TW,
        (23) -> X, SX, TX,
    }
    25 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
        (19) -> T, ST, TT,
        (20) -> U, SU, TU,
        (21) -> V, SV, TV,
        (22) -> W, SW, TW,
        (23) -> X, SX, TX,
        (24) -> Y, SY, TY,
    }
    26 {
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
        (12) -> M, SM, TM,
        (13) -> N, SN, TN,
        (14) -> O, SO, TO,
        (15) -> P, SP, TP,
        (16) -> Q, SQ, TQ,
        (17) -> R, SR, TR,
        (18) -> S, SS, TS,
        (19) -> T, ST, TT,
        (20) -> U, SU, TU,
        (21) -> V, SV, TV,
        (22) -> W, SW, TW,
        (23) -> X, SX, TX,
        (24) -> Y, SY, TY,
        (25) -> Z, SZ, TZ,
    }
}
