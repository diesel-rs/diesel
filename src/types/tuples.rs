use super::{NativeSqlType, FromSql};
use {Queriable, Table, Column};
use row::Row;

// FIXME(https://github.com/rust-lang/rust/issues/19630) Remove this work-around
macro_rules! e {
    ($e:expr) => { $e }
}

macro_rules! tuple_impls {
    ($(
        $Tuple:ident {
            $(($idx:tt) -> $T:ident, $ST:ident,)+
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

            impl<$($T),+, $($ST),+, SourceTable>
                Column<($($ST),+), SourceTable> for ($($T),+) where
                $($T: Column<$ST, SourceTable>),+,
                $($ST: NativeSqlType),+,
                SourceTable: Table,
            {
                #[allow(non_snake_case)]
                fn name(&self) -> String {
                    let parts: &[String] = e!(&[$(self.$idx.name()),*]);
                    parts.join(", ")
                }
            }
        )+
    }
}

tuple_impls! {
    T2 {
        (0) -> A, SA,
        (1) -> B, SB,
    }
    T3 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
    }
    T4 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
    }
    T5 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
    }
    T6 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
        (5) -> F, SF,
    }
    T7 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
        (5) -> F, SF,
        (6) -> G, SG,
    }
    T8 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
        (5) -> F, SF,
        (6) -> G, SG,
        (7) -> H, SH,
    }
    T9 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
        (5) -> F, SF,
        (6) -> G, SG,
        (7) -> H, SH,
        (8) -> I, SI,
    }
    T10 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
        (5) -> F, SF,
        (6) -> G, SG,
        (7) -> H, SH,
        (8) -> I, SI,
        (9) -> J, SJ,
    }
    T11 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
        (5) -> F, SF,
        (6) -> G, SG,
        (7) -> H, SH,
        (8) -> I, SI,
        (9) -> J, SJ,
        (10) -> K, SK,
    }
    T12 {
        (0) -> A, SA,
        (1) -> B, SB,
        (2) -> C, SC,
        (3) -> D, SD,
        (4) -> E, SE,
        (5) -> F, SF,
        (6) -> G, SG,
        (7) -> H, SH,
        (8) -> I, SI,
        (9) -> J, SJ,
        (10) -> K, SK,
        (11) -> L, SL,
    }
}
