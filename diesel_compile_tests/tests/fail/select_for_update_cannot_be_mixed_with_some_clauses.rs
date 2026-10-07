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
    //~^ ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: DistinctDsl`
    //~| ERROR: the trait bound `SelectStatement<FromClause<_>>: DistinctDsl` is not satisfied
    users.distinct().for_update();
    //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _>: LockingDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<_>>: LockingDsl<_>` is not satisfied
    users.for_update().distinct_on(id);
    //~^ ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: DistinctOnDsl<id>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: DistinctOnDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: Table` is not satisfied
    users.distinct_on(id).for_update();
    //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _>: LockingDsl<_>` is not satisfied
    //~| ERROR: SelectStatement<FromClause<_>>: LockingDsl<_>
    users.for_update().group_by(id);
    //~^ ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<_>>: GroupByDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: GroupByDsl<_>` is not satisfied
    users.group_by(id).for_update();
    //~^ ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<_>>: LockingDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _>: LockingDsl<_>` is not satisfied
    users.into_boxed().for_update();
    //~^ ERROR: the trait bound `BoxedSelectStatement<'_, (Integer,), _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<_>>: LockingDsl<_>` is not satisfied
    //~| ERROR: the trait bound `BoxedSelectStatement<'_, _, _, _>: LockingDsl<_>` is not satisfied
    users.for_update().into_boxed();
    //~^ ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: Table` is not satisfied
    //~| ERROR: cannot box `SelectStatement<_, _, _, _, _, _, _, _, _>` for backend `_`
    //~| ERROR: cannot box `SelectStatement<_, _, _, _, _, _, _, _, _>` for backend `_`
    users.for_update().group_by(id).having(id.gt(1));
    //~^ ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<_>>: GroupByDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _, _>: GroupByDsl<_>` is not satisfied
    users.group_by(id).having(id.gt(1)).for_update();
    //~^ ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _>: Table` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<_>>: LockingDsl<_>` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<_, _, _, _, _, _, _, _>: LockingDsl<_>` is not satisfied
}
