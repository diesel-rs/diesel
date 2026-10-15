use crate::schema::posts::dsl::{posts, title};
use crate::schema::users::dsl::*;
use crate::schema::{TestBackend, TestConnection, connection};
use diesel::query_builder::{AsQuery, AstPass, Query, QueryFragment, QueryId};
use diesel::sql_types::{Bool, Integer};
use diesel::*;
use std::cmp::max;
use std::mem;

#[diesel_test_helper::test]
fn complex_queries_with_no_data_have_no_size() {
    assert_eq!(0, mem::size_of_val(&users.as_query()));
    assert_eq!(0, mem::size_of_val(&users.select(id).as_query()));
    assert_eq!(
        0,
        mem::size_of_val(&users.inner_join(posts).filter(name.eq(title)))
    );
}

#[diesel_test_helper::test]
#[allow(clippy::size_of_ref)] // clippy is wrong here these tests fail otherwise
fn queries_with_data_are_no_bigger_than_their_variable_data() {
    assert_eq!(
        mem::size_of_val(&"Sean"),
        mem::size_of_val(&users.inner_join(posts).filter(name.eq("Sean")))
    );
    assert_eq!(
        mem::size_of::<i32>(),
        mem::size_of_val(&users.inner_join(posts).filter(id.eq(1)))
    );
    let source = users
        .inner_join(posts)
        .filter(name.eq("Sean"))
        .filter(id.eq(1));
    assert_eq!(
        mem::size_of_val(&"Sean") + max(mem::align_of_val(&source), mem::size_of::<i32>()),
        mem::size_of_val(&source)
    );
    let source = users
        .inner_join(posts)
        .filter(name.eq("Sean").and(id.eq(1)));
    assert_eq!(
        mem::size_of_val(&"Sean") + max(mem::align_of_val(&source), mem::size_of::<i32>()),
        mem::size_of_val(&source)
    );
}

#[diesel_test_helper::test]
fn wrong_query_id_impl() {
    #[derive(QueryId)]
    struct InList(Vec<i32>);

    impl QueryFragment<TestBackend> for InList {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, TestBackend>) -> QueryResult<()> {
            out.push_sql("SELECT 1 IN (");
            for (i, v) in self.0.iter().enumerate() {
                if i > 0 {
                    out.push_sql(", ");
                }
                out.push_bind_param::<Integer, _>(v)?;
            }
            out.push_sql(")");
            Ok(())
        }
    }
    impl RunQueryDsl<TestConnection> for InList {}

    impl Query for InList {
        type SqlType = Bool;
    }

    let conn = &mut connection();
    let r = InList(vec![2]).get_result::<bool>(conn).unwrap();
    assert!(!r);
    let r = InList((1..=64).collect()).get_result::<bool>(conn);
    assert!(r.is_err(), "{r:?}");
    assert_eq!(
        r.unwrap_err().to_string(),
        "cached statement expected a different number of parameters than provided: Expected 1, provided: 64"
    );
}
