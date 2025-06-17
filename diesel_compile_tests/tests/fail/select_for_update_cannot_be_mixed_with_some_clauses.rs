extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    users.for_update().distinct();
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: DistinctDsl` is not satisfied
    users.distinct().for_update();
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
    users.for_update().distinct_on(id);
    //~^ ERROR: the trait bound `SelectStatement<FromClause<...>>: DistinctOnDsl<_>` is not satisfied
    //~| ERROR: Cannot select `columns::id` from `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>`
    //~| ERROR: type mismatch resolving `<SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: DistinctOnDsl<_>` is not satisfied
    users.distinct_on(id).for_update();
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied

    users.for_update().group_by(id);
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: GroupByDsl<_>` is not satisfied
    users.group_by(id).for_update();
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ..., ..., ..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>, ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied

    users.into_boxed().for_update();
    //~^ ERROR: type mismatch resolving `<BoxedSelectStatement<'_, ..., ..., _> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `BoxedSelectStatement<'_, (diesel::sql_types::Integer,), FromClause<users::table>, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
    users.for_update().into_boxed();
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: BoxedDsl<'_, _>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: BoxedDsl<'_, _>` is not satisfied

    users.for_update().group_by(id).having(id.gt(1));
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: GroupByDsl<_>` is not satisfied
    users.group_by(id).having(id.gt(1)).for_update();
    //~^ ERROR: type mismatch resolving `<SelectStatement<..., ..., ..., ..., ..., ..., ..., ...> as AsQuery>::Query == SelectStatement<...>`
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
}
