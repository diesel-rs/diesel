use backend::{Backend, SupportsDefaultKeyword};
use expression::{Expression, SelectableExpression, NonAggregate};
use persistable::{ColumnInsertValue, InsertValues};
use query_builder::{Changeset, AsChangeset, QueryBuilder, BuildQueryResult, QueryFragment};
use query_source::{QuerySource, Queryable, Table, Column};
use row::Row;
use std::error::Error;
use types::{HasSqlType, FromSqlRow, ToSql, Nullable, IntoNullable, NotNull};

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
            impl<$($T),+, DB> HasSqlType<($($T,)+)> for DB where
                $(DB: HasSqlType<$T>),+,
                DB: Backend,
            {
                fn metadata() -> DB::TypeMetadata {
                    unreachable!("Tuples should never implement `ToSql` directly");
                }
            }

            impl<$($T),+> NotNull for ($($T,)+) {
            }

            impl<$($T),+, $($ST),+, DB> FromSqlRow<($($ST,)+), DB> for ($($T,)+) where
                DB: Backend,
                $($T: FromSqlRow<$ST, DB>),+,
                $(DB: HasSqlType<$ST>),+,
                DB: HasSqlType<($($ST,)+)>,
            {
                fn build_from_row<RowT: Row<DB>>(row: &mut RowT) -> Result<Self, Box<Error>> {
                    Ok(($(try!($T::build_from_row(row)),)+))
                }
            }

            impl<$($T),+, $($ST),+, DB> FromSqlRow<Nullable<($($ST,)+)>, DB> for Option<($($T,)+)> where
                DB: Backend,
                $($T: FromSqlRow<$ST, DB>),+,
                $(DB: HasSqlType<$ST>),+,
                DB: HasSqlType<($($ST,)+)>,
            {
                fn build_from_row<RowT: Row<DB>>(row: &mut RowT) -> Result<Self, Box<Error>> {
                    if e!(row.next_is_null($Tuple)) {
                        Ok(None)
                    } else {
                        Ok(Some(($(try!($T::build_from_row(row)),)+)))
                    }
                }
            }

            impl<$($T),+, $($ST),+, DB> Queryable<($($ST,)+), DB> for ($($T,)+) where
                DB: Backend,
                $($T: Queryable<$ST, DB>),+,
                $(DB: HasSqlType<$ST>),+,
                DB: HasSqlType<($($ST,)+)>,
            {
                type Row = ($($T::Row,)+);

                fn build(row: Self::Row) -> Self {
                    ($($T::build(e!(row.$idx)),)+)
                }
            }

            impl<$($T: Expression + NonAggregate),+> Expression for ($($T,)+) {
                type SqlType = ($(<$T as Expression>::SqlType),+);
            }

            impl<$($T: QueryFragment<DB>),+, DB: Backend> QueryFragment<DB> for ($($T,)+) {
                fn to_sql(&self, out: &mut DB::QueryBuilder)
                -> BuildQueryResult {
                    $(
                        if e!($idx) != 0 {
                            out.push_sql(", ");
                        }
                        try!(e!(self.$idx.to_sql(out)));
                    )+
                    Ok(())
                }
            }

            impl<$($T: Expression + NonAggregate),+> NonAggregate for ($($T,)+) {
            }

            impl<$($T,)+ $($ST,)+ Tab, DB> InsertValues<DB>
                for ($(ColumnInsertValue<$T, $ST>,)+) where
                    DB: Backend + SupportsDefaultKeyword,
                    Tab: Table,
                    $($T: Column<Table=Tab>,)+
                    $($ST: Expression<SqlType=$T::SqlType> + QueryFragment<DB>,)+
            {
                fn column_names(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
                    $(
                        if e!($idx) != 0 {
                            out.push_sql(", ");
                        }
                        try!(out.push_identifier($T::name()));
                    )+
                    Ok(())
                }

                fn values_clause(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
                    out.push_sql("(");
                    $(
                        if e!($idx) != 0 {
                            out.push_sql(", ");
                        }
                        match e!(&self.$idx) {
                            &ColumnInsertValue::Expression(_, ref value) => {
                                try!(value.to_sql(out));
                            }
                            _ => out.push_sql("DEFAULT"),
                        }
                    )+
                    out.push_sql(")");
                    Ok(())
                }
            }

            #[cfg(feature = "sqlite")]
            impl<$($T,)+ $($ST,)+ Tab> InsertValues<::sqlite::Sqlite>
                for ($(ColumnInsertValue<$T, $ST>,)+) where
                    Tab: Table,
                    $($T: Column<Table=Tab>,)+
                    $($ST: Expression<SqlType=$T::SqlType> + QueryFragment<::sqlite::Sqlite>,)+
            {
                #[allow(unused_assignments)]
                fn column_names(&self, out: &mut ::sqlite::SqliteQueryBuilder) -> BuildQueryResult {
                    let mut columns_present = false;
                    $(
                        match e!(&self.$idx) {
                            &ColumnInsertValue::Expression(..) => {
                                if columns_present {
                                    out.push_sql(", ");
                                }
                                try!(out.push_identifier($T::name()));
                                columns_present = true;
                            }
                            _ => {}
                        }
                    )+
                    Ok(())
                }

                #[allow(unused_assignments)]
                fn values_clause(&self, out: &mut ::sqlite::SqliteQueryBuilder) -> BuildQueryResult {
                    out.push_sql("(");
                    let mut columns_present = false;
                    $(
                        match e!(&self.$idx) {
                            &ColumnInsertValue::Expression(_, ref value) => {
                                if columns_present {
                                    out.push_sql(", ");
                                }
                                try!(value.to_sql(out));
                                columns_present = true;
                            }
                            _ => {}
                        }
                    )+
                    out.push_sql(")");
                    Ok(())
                }
            }

            impl<$($T),+, $($ST),+, QS>
                SelectableExpression<QS, ($($ST,)+)>
                for ($($T,)+) where
                $($T: SelectableExpression<QS, $ST>),+,
                ($($T,)+): Expression,
            {
            }

            impl<$($T),+, $($ST),+, QS>
                SelectableExpression<QS, Nullable<($($ST,)+)>>
                for ($($T,)+) where
                $($ST: IntoNullable,)+
                $($T: SelectableExpression<QS, $ST::Nullable, SqlType=$ST>),+,
                ($($T,)+): Expression,
            {
            }

            impl<Target, $($T,)+> AsChangeset for ($($T,)+) where
                $($T: AsChangeset<Target=Target>,)+
                Target: QuerySource,
            {
                type Target = Target;
                type Changeset = ($($T::Changeset,)+);

                fn as_changeset(self) -> Self::Changeset {
                    ($(e!(self.$idx.as_changeset()),)+)
                }
            }

            impl<DB, $($T,)+> Changeset<DB> for ($($T,)+) where
                DB: Backend,
                $($T: Changeset<DB>,)+
            {
                fn is_noop(&self) -> bool {
                    $(e!(self.$idx.is_noop()) &&)+ true
                }

                #[allow(unused_assignments)]
                fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
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
    1 {
        (0) -> A, SA, TA,
    }
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
