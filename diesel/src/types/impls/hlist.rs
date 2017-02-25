use backend::Backend;
use hlist::*;
use query_builder::*;
use query_source::Queryable;
use row::Row;
use types::{HasSqlType, FromSqlRow, Nullable, NotNull};

impl<Head, Tail, DB> HasSqlType<Cons<Head, Tail>> for DB where
    DB: Backend + HasSqlType<Head> + HasSqlType<Tail>,
{
    fn metadata() -> DB::TypeMetadata {
        unreachable!("hlists don't implement `ToSql` directly");
    }

    fn row_metadata(out: &mut Vec<DB::TypeMetadata>) {
        <DB as HasSqlType<Head>>::row_metadata(out);
        <DB as HasSqlType<Tail>>::row_metadata(out);
    }
}

impl<DB: Backend> HasSqlType<Nil> for DB {
    fn metadata() -> DB::TypeMetadata {
        unreachable!("hlists don't implement `ToSql` directly");
    }

    fn row_metadata(_: &mut Vec<DB::TypeMetadata>) {
        // noop
    }
}

impl<Head, Tail> NotNull for Cons<Head, Tail> {
}

impl<Head, Tail, HeadST, TailST, DB> FromSqlRow<Cons<HeadST, TailST>, DB>
    for Cons<Head, Tail> where
        Head: FromSqlRow<HeadST, DB>,
        Tail: FromSqlRow<TailST, DB>,
        DB: Backend + HasSqlType<HeadST> + HasSqlType<TailST>,
{
    fn build_from_row<R: Row<DB>>(row: &mut R) -> BuildQueryResult<Self> {
        Ok(Cons(
            try!(Head::build_from_row(row)),
            try!(Tail::build_from_row(row)),
        ))
    }
}

impl<DB: Backend> FromSqlRow<Nil, DB> for Nil {
    fn build_from_row<R: Row<DB>>(_: &mut R) -> BuildQueryResult<Self> {
        Ok(Nil)
    }
}

impl<Head, Tail, ST, DB> FromSqlRow<Nullable<ST>, DB>
    for Option<Cons<Head, Tail>> where
        DB: Backend + HasSqlType<ST>,
        Cons<Head, Tail>: FromSqlRow<ST, DB> + Hlist,
        ST: NotNull,
{
    fn build_from_row<R: Row<DB>>(row: &mut R) -> BuildQueryResult<Self> {
        if row.next_is_null(Cons::<Head, Tail>::len()) {
            Ok(None)
        } else {
            Cons::<Head, Tail>::build_from_row(row).map(Some)
        }
    }
}

impl<Head, Tail, HeadST, TailST, DB> Queryable<Cons<HeadST, TailST>, DB>
    for Cons<Head, Tail> where
        DB: Backend + HasSqlType<HeadST> + HasSqlType<TailST>,
        Head: Queryable<HeadST, DB>,
        Tail: Queryable<TailST, DB>,
{
    type Row = Cons<Head::Row, Tail::Row>;

    fn build(Cons(head, tail): Self::Row) -> Self {
        Cons(
            Head::build(head),
            Tail::build(tail),
        )
    }
}

impl<DB: Backend> Queryable<Nil, DB> for Nil {
    type Row = Nil;

    fn build(_: Self::Row) -> Self {
        Nil
    }
}
