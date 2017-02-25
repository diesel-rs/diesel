use associations::BelongsTo;
use backend::Backend;
use query_source::Queryable;
use types::HasSqlType;

macro_rules! tuple_impls {
    ($($T:ident,)+) => {
        impl<$($T,)+ ST, DB> Queryable<ST, DB> for ($($T,)+) where
            DB: Backend + HasSqlType<ST>,
            Hlist!($($T,)+): Queryable<ST, DB>,
        {
            type Row = <Hlist!($($T,)+) as Queryable<ST, DB>>::Row;

            #[allow(non_snake_case)]
            fn build(row: Self::Row) -> Self {
                let hlist_pat!($($T,)+) = Queryable::build(row);
                ($($T,)+)
            }
        }

        impl<$($T,)+ Parent> BelongsTo<Parent> for ($($T,)+) where
            A: BelongsTo<Parent>,
        {
            type ForeignKey = A::ForeignKey;
            type ForeignKeyColumn = A::ForeignKeyColumn;

            fn foreign_key(&self) -> Option<&Self::ForeignKey> {
                self.0.foreign_key()
            }

            fn foreign_key_column() -> Self::ForeignKeyColumn {
                A::foreign_key_column()
            }
        }
    }
}

tuple_impls!(A,);
tuple_impls!(A, B,);
tuple_impls!(A, B, C,);
tuple_impls!(A, B, C, D,);
tuple_impls!(A, B, C, D, E,);
tuple_impls!(A, B, C, D, E, F,);
tuple_impls!(A, B, C, D, E, F, G,);
tuple_impls!(A, B, C, D, E, F, G, H,);
tuple_impls!(A, B, C, D, E, F, G, H, I,);
tuple_impls!(A, B, C, D, E, F, G, H, I, J,);
tuple_impls!(A, B, C, D, E, F, G, H, I, J, K,);
tuple_impls!(A, B, C, D, E, F, G, H, I, J, K, L,);
