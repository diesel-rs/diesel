use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(test)]
use std::ffi::CString;
use std::rc::Rc;

use result::QueryResult;
use super::{Query, RawConnection};

pub struct StatementCache {
    cache: RefCell<HashMap<StatementCacheKey, Rc<Query>>>,
}

#[derive(Hash, PartialEq, Eq)]
struct StatementCacheKey {
    sql: String,
    bind_types: Vec<u32>,
}

impl StatementCache {
    pub fn new() -> Self {
        StatementCache {
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn cached_query(
        &self,
        conn: &RawConnection,
        sql: String,
        bind_types: Vec<u32>,
    ) -> QueryResult<Rc<Query>> {
        let mut cache = self.cache.borrow_mut();
        let cache_key = StatementCacheKey {
            sql: sql,
            bind_types: bind_types
        };

        // FIXME: This can be cleaned up once https://github.com/rust-lang/rust/issues/32281
        // is stable
        if cache.contains_key(&cache_key) {
            Ok(cache[&cache_key].clone())
        } else {
            let name = format!("__diesel_stmt_{}", cache.len() + 1);
            let statement = Rc::new(try!(Query::prepare(
                conn,
                &cache_key.sql,
                &name,
                &cache_key.bind_types,
            )));
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
