use crate::backend::{Backend, SupportsOnConflictClause};
use crate::expression::SqlLiteral;
use crate::query_builder::*;
use crate::query_source::Column;
use crate::result::QueryResult;

/// Used to specify the constraint name for an upsert statement in the form `ON
/// CONFLICT ON CONSTRAINT`. Note that `constraint_name` must be the name of a
/// unique constraint, not the name of an index.
///
/// # Example
///
/// ```rust
/// # include!("on_conflict_docs_setup.rs");
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// use diesel::upsert::*;
///
/// #     let conn = establish_connection();
/// #     conn.execute("TRUNCATE TABLE users").unwrap();
/// conn.execute("ALTER TABLE users ADD CONSTRAINT users_name UNIQUE (name)").unwrap();
/// let user = User { id: 1, name: "Sean", };
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
    OnConstraint {
        constraint_name: constraint_name,
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConstraint<'a> {
    constraint_name: &'a str,
}

pub trait OnConflictTarget<Table> {}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct NoConflictTarget;

impl<DB: Backend + SupportsOnConflictClause> QueryFragment<DB> for NoConflictTarget {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<Table> OnConflictTarget<Table> for NoConflictTarget {}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct ConflictTarget<T>(pub T);

impl<DB, T> QueryFragment<DB> for ConflictTarget<T>
where
    DB: Backend + SupportsOnConflictClause,
    T: Column,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" (");
        out.push_identifier(T::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T> OnConflictTarget<T::Table> for ConflictTarget<T>
where
    T: Column,
{}

impl<DB, ST> QueryFragment<DB> for ConflictTarget<SqlLiteral<ST>>
where
    DB: Backend + SupportsOnConflictClause,
    SqlLiteral<ST>: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Tab, ST> OnConflictTarget<Tab> for ConflictTarget<SqlLiteral<ST>> {}

impl<'a, DB> QueryFragment<DB> for ConflictTarget<OnConstraint<'a>>
where
    DB: Backend + SupportsOnConflictClause,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" ON CONSTRAINT ");
        out.push_identifier(self.0.constraint_name)?;
        Ok(())
    }
}

impl<'a, Table> OnConflictTarget<Table> for ConflictTarget<OnConstraint<'a>> {}

macro_rules! on_conflict_tuples {
    ($($col:ident),+) => {
        impl<DB, T, $($col),+> QueryFragment<DB> for ConflictTarget<(T, $($col),+)> where
            DB: Backend + SupportsOnConflictClause,
            T: Column,
            $($col: Column<Table=T::Table>,)+
        {
            fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
                out.push_sql(" (");
                out.push_identifier(T::NAME)?;
                $(
                    out.push_sql(", ");
                    out.push_identifier($col::NAME)?;
                )+
                out.push_sql(")");
                Ok(())
            }
        }

        impl<T, $($col),+> OnConflictTarget<T::Table> for ConflictTarget<(T, $($col),+)> where
            T: Column,
            $($col: Column<Table=T::Table>,)+
        {
        }
    }
}

on_conflict_tuples!(U);
on_conflict_tuples!(U, V);
on_conflict_tuples!(U, V, W);
on_conflict_tuples!(U, V, W, X);
on_conflict_tuples!(U, V, W, X, Y);
on_conflict_tuples!(U, V, W, X, Y, Z);
