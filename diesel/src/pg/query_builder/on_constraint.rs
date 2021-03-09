use crate::pg::Pg;
use crate::query_builder::upsert::on_conflict_target::{ConflictTarget, OnConflictTarget};
use crate::query_builder::*;
use crate::result::QueryResult;

/// Used to specify the constraint name for an upsert statement in the form `ON
/// CONFLICT ON CONSTRAINT`. Note that `constraint_name` must be the name of a
/// unique constraint, not the name of an index.
///
/// # Example
///
/// ```rust
/// # extern crate diesel;
/// # include!("../../upsert/on_conflict_docs_setup.rs");
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// use diesel::upsert::*;
///
/// #     let conn = establish_connection();
/// #     conn.execute("TRUNCATE TABLE users").unwrap();
/// conn.execute("ALTER TABLE users ADD CONSTRAINT users_name UNIQUE (name)").unwrap();
/// let user = User { id: 1, name: "Sean" };
/// let same_name_different_id = User { id: 2, name: "Sean" };
/// let same_id_different_name = User { id: 1, name: "Pascal" };
///
/// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(&conn));
///
/// let inserted_row_count = diesel::insert_into(users)
///     .values(&same_name_different_id)
///     .on_conflict(on_constraint("users_name"))
///     .do_nothing()
///     .execute(&conn);
/// assert_eq!(Ok(0), inserted_row_count);
///
/// let pk_conflict_result = diesel::insert_into(users)
///     .values(&same_id_different_name)
///     .on_conflict(on_constraint("users_name"))
///     .do_nothing()
///     .execute(&conn);
/// assert!(pk_conflict_result.is_err());
/// # }
/// ```
pub fn on_constraint(constraint_name: &str) -> OnConstraint {
    OnConstraint { constraint_name }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConstraint<'a> {
    constraint_name: &'a str,
}

impl<'a> QueryFragment<Pg> for ConflictTarget<OnConstraint<'a>> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" ON CONSTRAINT ");
        out.push_identifier(self.0.constraint_name)?;
        Ok(())
    }
}

impl<'a, Table> OnConflictTarget<Table> for ConflictTarget<OnConstraint<'a>> {}
