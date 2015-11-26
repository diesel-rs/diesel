use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::{QuerySource, Table};

pub trait UpdateTarget: QuerySource {
    type Table: Table;

    fn where_clause(&self, out: &mut QueryBuilder) -> BuildQueryResult;
    fn table(&self) -> &Self::Table;
}
