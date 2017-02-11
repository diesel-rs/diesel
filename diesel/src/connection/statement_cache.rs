use std::any::TypeId;
use std::borrow::Cow;

use backend::Backend;
use query_builder::*;
use result::QueryResult;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
#[derive(Hash, PartialEq, Eq)]
pub enum StatementCacheKey<DB: Backend> {
    Type(TypeId),
    Sql {
        sql: String,
        bind_types: Vec<DB::TypeMetadata>,
    }
}

impl<DB> StatementCacheKey<DB> where
    DB: Backend,
    DB::QueryBuilder: Default,
    DB::TypeMetadata: Clone,
{
    pub fn for_source<T>(source: &T, bind_types: &[DB::TypeMetadata])
        -> QueryResult<Self> where
            T: QueryFragment<DB> + QueryId,
    {
        match T::query_id() {
            Some(id) => Ok(StatementCacheKey::Type(id)),
            None => {
                let sql = try!(Self::construct_sql(source));
                Ok(StatementCacheKey::Sql {
                    sql: sql,
                    bind_types: bind_types.into(),
                })
            }
        }
    }

    pub fn sql<T: QueryFragment<DB>>(&self, source: &T) -> QueryResult<Cow<str>> {
        match *self {
            StatementCacheKey::Type(_) => Self::construct_sql(source).map(Cow::Owned),
            StatementCacheKey::Sql { ref sql, .. } => Ok(Cow::Borrowed(sql)),
        }
    }

    fn construct_sql<T: QueryFragment<DB>>(source: &T) -> QueryResult<String> {
        use result::Error::QueryBuilderError;

        let mut query_builder = DB::QueryBuilder::default();
        try!(source.to_sql(&mut query_builder).map_err(QueryBuilderError));
        Ok(query_builder.finish())
    }
}
