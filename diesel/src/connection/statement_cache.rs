use std::any::TypeId;
use std::borrow::Cow;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

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
    ) -> QueryResult<MaybeCached<Statement>> where
        T: QueryFragment<DB> + QueryId,
        F: FnOnce(&str) -> QueryResult<Statement>,
    {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let cache_key = try!(StatementCacheKey::for_source(source, bind_types));

        if !source.is_safe_to_cache_prepared() {
            let sql = try!(cache_key.sql(source));
            return prepare_fn(&sql).map(MaybeCached::CannotCache)
        }

        refmut_map_result(self.cache.borrow_mut(), |cache| {
            match cache.entry(cache_key) {
                Occupied(entry) => Ok(entry.into_mut()),
                Vacant(entry) => {
                    let statement = {
                        let sql = try!(entry.key().sql(source));
                        prepare_fn(&sql)
                    };

                    Ok(entry.insert(try!(statement)))
                }
            }
        }).map(MaybeCached::Cached)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub enum MaybeCached<'a, T: 'a> {
    CannotCache(T),
    Cached(RefMut<'a, T>),
}

impl<'a, T> Deref for MaybeCached<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match *self {
            MaybeCached::CannotCache(ref x) => x,
            MaybeCached::Cached(ref x) => &**x,
        }
    }
}

impl<'a, T> DerefMut for MaybeCached<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match *self {
            MaybeCached::CannotCache(ref mut x) => x,
            MaybeCached::Cached(ref mut x) => &mut **x,
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

fn refmut_map_result<T, U, F>(refmut: RefMut<T>, mapper: F)
    -> QueryResult<RefMut<U>> where
        F: FnOnce(&mut T) -> QueryResult<&mut U>,
{
    use std::mem;

    let mut error = None;
    let result = RefMut::map(refmut, |mutref| match mapper(mutref) {
        Ok(x) => x,
        Err(e) => {
            error = Some(e);
            unsafe { mem::uninitialized() }
        }
    });
    match error {
        Some(e) => Err(e),
        None => Ok(result),
    }
}
