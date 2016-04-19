use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(test)]
use std::ffi::CString;
use std::rc::Rc;

use pg::{Pg, PgQueryBuilder};
use query_builder::{QueryFragment, QueryId};
use result::QueryResult;
use result::Error::QueryBuilderError;
use super::{Query, RawConnection};

pub struct StatementCache {
    cache: RefCell<HashMap<StatementCacheKey, Rc<Query>>>,
}

#[derive(Hash, PartialEq, Eq)]
enum StatementCacheKey {
    Type(TypeId),
    Query {
        sql: String,
        bind_types: Vec<u32>,
    }
}

impl StatementCacheKey {
    fn sql(&self) -> Option<&str> {
        match self {
            &StatementCacheKey::Query { ref sql, .. } => Some(&*sql),
            _ => None
        }
    }

    fn bind_types(&self) -> Option<&Vec<u32>> {
        match self {
            &StatementCacheKey::Query { ref bind_types, .. } => Some(bind_types),
            _ => None
        }
    }
}

impl StatementCache {
    pub fn new() -> Self {
        StatementCache {
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn cached_query<T: QueryFragment<Pg> + QueryId>(
        &self,
        conn: &Rc<RawConnection>,
        source: &T,
        bind_types: Vec<u32>,
    ) -> QueryResult<Rc<Query>> {
        use std::borrow::Cow;

        let mut cache = self.cache.borrow_mut();
        let (cache_key, maybe_binds) = try!(cache_key(conn, source, bind_types));
        // FIXME: This can be cleaned up once https://github.com/rust-lang/rust/issues/32281
        // is stable
        if cache.contains_key(&cache_key) {
            Ok(cache[&cache_key].clone())
        } else {
            let name = format!("__diesel_stmt_{}", cache.len() + 1);
            let statement = {
                let sql = match cache_key.sql() {
                    Some(sql) => Cow::Borrowed(sql),
                    None => Cow::Owned(try!(to_sql(conn, source))),
                };
                let bind_types = cache_key.bind_types()
                    .or(maybe_binds.as_ref()).unwrap();

                Rc::new(try!(Query::prepare(
                    conn,
                    &sql,
                    &name,
                    &bind_types,
                )))
            };
            let entry = cache.entry(cache_key);
            Ok(entry.or_insert(statement).clone())
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.cache.borrow().len()
    }

    #[cfg(test)]
    pub fn statement_names(&self) -> Vec<CString> {
        let mut statement_names = self.cache.borrow().iter()
            .map(|(_, v)| match **v {
                Query::Prepared { ref name, .. } => name.clone(),
                _ => unreachable!(),
            }).collect::<Vec<_>>();
        statement_names.dedup();
        statement_names
    }
}

fn to_sql<T: QueryFragment<Pg>>(conn: &Rc<RawConnection>, source: &T)
    -> QueryResult<String>
{
    let mut query_builder = PgQueryBuilder::new(conn);
    try!(source.to_sql(&mut query_builder).map_err(QueryBuilderError));
    Ok(query_builder.sql)
}

fn cache_key<T: QueryFragment<Pg> + QueryId>(
    conn: &Rc<RawConnection>,
    source: &T,
    bind_types: Vec<u32>,
) -> QueryResult<(StatementCacheKey, Option<Vec<u32>>)> {
    match T::query_id() {
        Some(id) => Ok((StatementCacheKey::Type(id), Some(bind_types))),
        None => Ok((
            StatementCacheKey::Query {
                sql: try!(to_sql(conn, source)),
                bind_types: bind_types
            },
            None,
        )),
    }
}
