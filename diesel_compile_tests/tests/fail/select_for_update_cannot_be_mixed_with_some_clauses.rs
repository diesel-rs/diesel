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
    //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: DistinctDsl`
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: DistinctDsl` is not satisfied
    users.distinct().for_update();
    //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ...>: LockingDsl<...>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
    users.for_update().distinct_on(id);
    //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: DistinctOnDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: DistinctOnDsl<...>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: DistinctOnDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    users.distinct_on(id).for_update();
    //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ...>: LockingDsl<...>` is not satisfied
    users.for_update().group_by(id);
    //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: GroupByDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: GroupByDsl<_>` is not satisfied
    users.group_by(id).for_update();
    //~^ ERROR: the trait bound `SelectStatement<FromClause<...>, ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ...>: LockingDsl<...>` is not satisfied
    users.into_boxed().for_update();
    //~^ ERROR: the trait bound `BoxedSelectStatement<'_, (diesel::sql_types::Integer,), FromClause<users::table>, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
    //~| ERROR: the trait bound `BoxedSelectStatement<'_, (Integer,), ..., _>: LockingDsl<...>` is not satisfied
    users.for_update().into_boxed();
    //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: cannot box `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>` for backend `_`
    //~| ERROR: cannot box `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>` for backend `_`
    users.for_update().group_by(id).having(id.gt(1));
    //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: GroupByDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ..., ...>: GroupByDsl<_>` is not satisfied
    //~| ERROR: type annotations needed
    users.group_by(id).having(id.gt(1)).for_update();
    //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ...>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<...>>: LockingDsl<...>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ..., ...>: LockingDsl<...>` is not satisfied
}
