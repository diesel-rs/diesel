use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::{QuerySource, Table};

pub trait UpdateTarget: QuerySource {
    type Table: Table;

    fn where_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult;
    fn table(&self) -> &Self::Table;
}
