use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::{QuerySource, Table};

/// You should not need to implement this trait.
/// [`table!`](../macro.table!.html) will implement it for you.
///
/// Types which can be passed to [`update`](fn.update.html). This will be
/// implemented for [tables](../query_source/trait.Table.html), and the result
/// of calling [`filter`](../query_dsl/trait.FilterDsl.html).
///
/// Errors about this trait not being implemented are likely indicating that you
/// have called a method like `select` or `order`, which does not make sense in
/// the context of an `update` or `delete` operation.
pub trait UpdateTarget: QuerySource {
    type Table: Table;

    fn where_clause(&self, out: &mut QueryBuilder) -> BuildQueryResult;
}
