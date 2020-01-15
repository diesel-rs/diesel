use crate::schema::posts::dsl::{posts, title};
use crate::schema::users::dsl::*;
use diesel::query_builder::AsQuery;
use diesel::*;
use std::cmp::max;
use std::mem;

#[test]
fn complex_queries_with_no_data_have_no_size() {
    assert_eq!(0, mem::size_of_val(&users.as_query()));
    assert_eq!(0, mem::size_of_val(&users.select(id).as_query()));
    assert_eq!(
        0,
        mem::size_of_val(&users.inner_join(posts).filter(name.eq(title)))
    );
}

#[test]
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
