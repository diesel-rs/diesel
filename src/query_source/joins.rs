use {QuerySource, Table};
use query_builder::{QueryBuilder, BuildQueryResult};
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
    type SqlType = (Left::SqlType, Right::SqlType);

    fn select_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        try!(self.left.select_clause(out));
        out.push_sql(", ");
        self.right.select_clause(out)
    }

    fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        try!(self.left.from_clause(out));
        out.push_sql(" INNER JOIN ");
        try!(self.right.from_clause(out));
        out.push_sql(" ON ");
        out.push_sql(&self.left.join_sql());
        Ok(())
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
    type SqlType = (Left::SqlType, Nullable<Right::SqlType>);

    fn select_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        try!(self.left.select_clause(out));
        out.push_sql(", ");
        self.right.select_clause(out)
    }

    fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        try!(self.left.from_clause(out));
        out.push_sql(" LEFT OUTER JOIN ");
        try!(self.right.from_clause(out));
        out.push_sql(" ON ");
        out.push_sql(&self.left.join_sql());
        Ok(())
    }
}

pub trait JoinTo<T: Table>: Table {
    fn join_sql(&self) -> String;
}
