use backend::Backend;
use expression::*;
use query_builder::*;
use result::QueryResult;

pub trait Collation {
    fn get_collation<'a>(&'a self) -> &'a str;
}

#[derive(Debug, Clone, Copy)]
pub struct Collate<Lhs: Expression, Coll: Collation> {
    left: Lhs,
    collation: Coll,
}

impl<Lhs: Expression, Coll: Collation> Collate<Lhs, Coll> {
    pub fn new(left: Lhs, collation: Coll) -> Self {
        Collate { left, collation }
    }
}

impl<Lhs, Coll> QueryId for Collate<Lhs, Coll>
where
    Lhs: 'static + Expression + QueryId,
    Coll: 'static + Collation,
{
    type QueryId = Collate<Lhs, Coll>;
    const HAS_STATIC_QUERY_ID: bool = Lhs::HAS_STATIC_QUERY_ID;
}

impl<Lhs, Coll, QS> SelectableExpression<QS> for Collate<Lhs, Coll>
where
    Collate<Lhs, Coll>: AppearsOnTable<QS>,
    Lhs: Expression + SelectableExpression<QS>,
    Coll: Collation,
{
}

impl<Lhs, Coll, QS> AppearsOnTable<QS> for Collate<Lhs, Coll>
where
    Collate<Lhs, Coll>: Expression,
    Lhs: Expression + AppearsOnTable<QS>,
    Coll: Collation,
{
}

impl<Lhs, Coll> Expression for Collate<Lhs, Coll>
where
    Lhs: Expression,
    Coll: Collation,
{
    type SqlType = Lhs::SqlType;
}

impl<Lhs, Coll> NonAggregate for Collate<Lhs, Coll>
where
    Lhs: Expression + NonAggregate,
    Coll: Collation,
{
}

impl<Lhs, Coll, DB> QueryFragment<DB> for Collate<Lhs, Coll>
where
    Lhs: Expression + QueryFragment<DB>,
    Coll: Collation,
    DB: Backend,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(" COLLATE ");
        out.push_sql(self.collation.get_collation());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CollationBinary;

impl Collation for CollationBinary {
    fn get_collation<'a>(&'a self) -> &'a str {
        "BINARY"
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CollationNoCase;

impl Collation for CollationNoCase {
    fn get_collation<'a>(&'a self) -> &'a str {
        "NOCASE"
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CollationRTrim;

impl Collation for CollationRTrim {
    fn get_collation<'a>(&'a self) -> &'a str {
        "RTRIM"
    }
}
