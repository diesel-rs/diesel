use {QuerySource, Table};
use query_builder::*;
use expression::SelectableExpression;
use types::Nullable;

#[derive(Clone, Copy)]
pub struct InnerJoinSource<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> InnerJoinSource<Left, Right> {
    pub fn new(left: Left, right: Right) -> Self {
        InnerJoinSource {
            left: left,
            right: right,
        }
    }
}

impl<Left, Right> QuerySource for InnerJoinSource<Left, Right> where
    Left: Table + JoinTo<Right>,
    Right: Table,
{
    fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        try!(self.left.from_clause(out));
        out.push_sql(" INNER JOIN ");
        try!(self.right.from_clause(out));
        out.push_sql(" ON ");
        out.push_sql(&self.left.join_sql());
        Ok(())
    }
}

impl<Left, Right> AsQuery for InnerJoinSource<Left, Right> where
    Left: Table + JoinTo<Right>,
    Right: Table,
    (Left::Star, Right::Star): SelectableExpression<
                                   InnerJoinSource<Left, Right>,
                                   (Left::SqlType, Right::SqlType),
                               >,
{
    type SqlType = (Left::SqlType, Right::SqlType);
    type Query = SelectStatement<
        (Left::SqlType, Right::SqlType),
        (Left::Star, Right::Star),
        Self,
    >;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple((self.left.star(), self.right.star()), self)
    }
}

#[derive(Clone, Copy)]
pub struct LeftOuterJoinSource<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> LeftOuterJoinSource<Left, Right> {
    pub fn new(left: Left, right: Right) -> Self {
        LeftOuterJoinSource {
            left: left,
            right: right,
        }
    }
}

impl<Left, Right> QuerySource for LeftOuterJoinSource<Left, Right> where
    Left: Table + JoinTo<Right>,
    Right: Table,
{
    fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        try!(self.left.from_clause(out));
        out.push_sql(" LEFT OUTER JOIN ");
        try!(self.right.from_clause(out));
        out.push_sql(" ON ");
        out.push_sql(&self.left.join_sql());
        Ok(())
    }
}

impl<Left, Right> AsQuery for LeftOuterJoinSource<Left, Right> where
    Left: Table + JoinTo<Right>,
    Right: Table,
    (Left::Star, Right::Star): SelectableExpression<
                                   LeftOuterJoinSource<Left, Right>,
                                   (Left::SqlType, Nullable<Right::SqlType>),
                               >,
{
    type SqlType = (Left::SqlType, Nullable<Right::SqlType>);
    type Query = SelectStatement<
        (Left::SqlType, Nullable<Right::SqlType>),
        (Left::Star, Right::Star),
        Self,
    >;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple((self.left.star(), self.right.star()), self)
    }
}

pub trait JoinTo<T: Table>: Table {
    fn join_sql(&self) -> String;
}
