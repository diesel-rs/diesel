#[cfg(doc)]
use crate::backend::SqlDialect;
use crate::backend::{Backend, sql_dialect};
use crate::query_builder::{AstPass, QueryFragment};
use crate::{QueryResult, query_builder::*};
use core::marker::PhantomData;

#[cfg(any(
    feature = "__sqlite-shared",
    feature = "postgres_backend",
    feature = "mysql_backend"
))]
pub(crate) const BATCH_UPDATE_ALIAS: &str = "__diesel_internal_temp_values";

#[cfg(any(
    feature = "__sqlite-shared",
    feature = "postgres_backend",
    feature = "mysql_backend"
))]
pub trait BatchValueHelper<DB: Backend> {
    fn assign<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;

    fn column_name<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;

    fn bind_value<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;
}

#[cfg(any(
    feature = "__sqlite-shared",
    feature = "postgres_backend",
    feature = "mysql_backend"
))]
pub trait BatchAssignHelper<DB: Backend> {
    fn batch_assign_identifier<'a>(&'a self, out: AstPass<'_, 'a, DB>) -> QueryResult<()>;
}

#[cfg(any(
    feature = "__sqlite-shared",
    feature = "postgres_backend",
    feature = "mysql_backend"
))]
pub trait BatchKeyHelper<PK, DB: Backend> {
    fn bind_value<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;

    fn column_name<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;

    fn assign<'b>(pk: &'b PK, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;
}

#[cfg(any(
    feature = "__sqlite-shared",
    feature = "postgres_backend",
    feature = "mysql_backend"
))]
impl<PK, DB, C> BatchKeyHelper<PK, DB> for C
where
    DB: Backend + crate::sql_types::HasSqlType<PK::SqlType>,
    C: crate::serialize::ToSql<PK::SqlType, DB>,
    PK: crate::expression::Expression + crate::Column + QueryFragment<DB>,
    PK::SqlType: crate::sql_types::SingleValue,
{
    fn bind_value<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_bind_param(self)
    }

    fn column_name<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_identifier(PK::NAME)
    }

    fn assign<'b>(pk: &'b PK, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pk.walk_ast(out.reborrow())?;
        out.push_sql(" = ");
        out.push_identifier(BATCH_UPDATE_ALIAS)?;
        out.push_sql(".");
        out.push_identifier(PK::NAME)?;
        Ok(())
    }
}

/// This type represents a batch update clause, which allows
/// to update multiple rows at once.
///
/// Custom backends can specialize the [`QueryFragment`]
/// implementation via [`SqlDialect::BatchUpdateSupport`]
/// or provide fully custom [`ExecuteDsl`](crate::query_dsl::methods::ExecuteDsl)
/// and [`LoadQuery`](crate::query_dsl::methods::LoadQuery) implementations
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
// warn(dead_code) is a false positive for the fields 'values' and 'primary_key' as
// specialized implementations for the backends actually use them.
#[allow(dead_code)]
#[derive(Debug)]
pub struct BatchUpdate<I, C, PK, Tab> {
    // values.0 -> I: Identifier from Identifiable::Id
    // values.1 -> V: Changeset from AsChangeset::Changeset
    pub(crate) values: Vec<(I, C)>,
    // PK: PrimaryKey will have same SqlType as I
    pub(crate) primary_key: PK,
    _marker: PhantomData<Tab>,
}

impl<I, C, PK, Tab> BatchUpdate<I, C, PK, Tab> {
    /// Docs
    pub fn new(values: Vec<(I, C)>, primary_key: PK) -> Self {
        Self {
            values,
            primary_key,
            _marker: PhantomData,
        }
    }
}

impl<I, C, PK, Tab, DB> QueryFragment<DB> for BatchUpdate<I, C, PK, Tab>
where
    DB: Backend,
    DB::BatchUpdateSupport: sql_dialect::batch_update_support::SupportsBatchUpdate,
    Self: QueryFragment<DB, DB::BatchUpdateSupport>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::BatchUpdateSupport>>::walk_ast(self, pass)
    }
}
