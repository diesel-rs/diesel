use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(test)]
use std::ffi::CString;
use std::rc::Rc;

use connection::StatementCacheKey;
use pg::{Pg, PgTypeMetadata};
use query_builder::{QueryFragment, QueryId};
use result::QueryResult;
use super::{Query, RawConnection};

pub struct StatementCache {
    cache: RefCell<HashMap<StatementCacheKey<Pg>, Rc<Query>>>,
}

impl StatementCache {
    pub fn new() -> Self {
        StatementCache {
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn cached_query<T: QueryFragment<Pg> + QueryId>(
        &self,
        conn: &RawConnection,
        source: &T,
        bind_types: &[PgTypeMetadata],
    ) -> QueryResult<Rc<Query>> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let cache_suffix = self.cache.borrow().len() + 1;
        let mut cache = self.cache.borrow_mut();
        let cache_key = try!(StatementCacheKey::for_source(source, bind_types));

        match cache.entry(cache_key) {
            Occupied(entry) => Ok(entry.get().clone()),
            Vacant(entry) => {
                let statement = {
                    let sql = try!(entry.key().sql(source));
                    let name = format!("__diesel_stmt_{}", cache_suffix);

                    Rc::new(try!(Query::prepare(
                        conn,
                        &sql,
                        &name,
                        bind_types,
                    )))
                };

                Ok(entry.insert(statement).clone())
            }
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
