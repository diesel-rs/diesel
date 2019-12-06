#![allow(unused_parens)] // FIXME: Remove this attribute once false positive is resolved.

use super::{PgConnection, PgTypeMetadata};
use prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;

/// Determines the OID of types at runtime
#[allow(missing_debug_implementations)]
#[repr(transparent)]
pub struct PgMetadataLookup {
    conn: PgConnection,
}

impl PgMetadataLookup {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(conn: &PgConnection) -> &Self {
        unsafe { &*(conn as *const PgConnection as *const PgMetadataLookup) }
    }

    /// Determine the type metadata for the given `type_name`
    ///
    /// This function should only be used for user defined types, or types which
    /// come from an extension. This function may perform a SQL query to look
    /// up the type. For built-in types, a static OID should be preferred.
    pub fn lookup_type(&self, type_name: &str) -> PgTypeMetadata {
        let metadata_cache = self.conn.get_metadata_cache();
        metadata_cache.lookup_type(type_name).unwrap_or_else(|| {
            use self::pg_type::dsl::*;

            let type_metadata = pg_type
                .select((oid, typarray))
                .filter(typname.eq(type_name))
                .first(&self.conn)
                .unwrap_or_default();
            metadata_cache.store_type(type_name, type_metadata);
            type_metadata
        })
    }
}

/// Stores a cache for the OID of custom types
#[allow(missing_debug_implementations)]
pub struct PgMetadataCache {
    cache: RefCell<HashMap<String, PgTypeMetadata>>,
}

impl PgMetadataCache {
    pub(crate) fn new() -> Self {
        PgMetadataCache {
            cache: RefCell::new(HashMap::new()),
        }
    }

    fn lookup_type(&self, type_name: &str) -> Option<PgTypeMetadata> {
        Some(*self.cache.borrow().get(type_name)?)
    }

    fn store_type(&self, type_name: &str, type_metadata: PgTypeMetadata) {
        self.cache
            .borrow_mut()
            .insert(type_name.to_owned(), type_metadata);
    }
}

table! {
    pg_type (oid) {
        oid -> Oid,
        typname -> Text,
        typarray -> Oid,
    }
}
