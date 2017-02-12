use std::any::TypeId;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;

use backend::Backend;
use query_builder::*;
use result::QueryResult;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct StatementCache<DB: Backend, Statement> {
    pub cache: RefCell<HashMap<StatementCacheKey<DB>, Statement>>,
}

#[cfg_attr(feature="clippy", allow(len_without_is_empty))]
impl<DB, Statement> StatementCache<DB, Statement> where
    DB: Backend,
    DB::TypeMetadata: Clone,
    DB::QueryBuilder: Default,
    StatementCacheKey<DB>: Hash + Eq,
{
    pub fn new() -> Self {
        StatementCache {
            cache: RefCell::new(HashMap::new())
        }
    }

    pub fn len(&self) -> usize {
        self.cache.borrow().len()
    }

    pub fn cached_statement<T, F>(
        &self,
        source: &T,
        bind_types: &[DB::TypeMetadata],
        prepare_fn: F,
    ) -> QueryResult<Statement> where
        T: QueryFragment<DB> + QueryId,
        F: FnOnce(&str) -> QueryResult<Statement>,
        Statement: Clone,
    {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let cache_key = try!(StatementCacheKey::for_source(source, bind_types));
        let mut cache = self.cache.borrow_mut();

        match cache.entry(cache_key) {
            Occupied(entry) => Ok(entry.get().clone()),
            Vacant(entry) => {
                let statement = {
                    let sql = try!(entry.key().sql(source));
                    prepare_fn(&sql)
                };

                if !source.is_safe_to_cache_prepared() {
                    return statement;
                }

                Ok(entry.insert(try!(statement)).clone())
            }
        }
    }
}

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
