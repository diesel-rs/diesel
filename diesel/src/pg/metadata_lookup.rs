#![allow(unused_parens)] // FIXME: Remove this attribute once false positive is resolved.

use super::{PgConnection, PgTypeMetadata};
use crate::prelude::*;

use std::borrow::Cow;
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
    pub fn lookup_type(&self, type_name: &str, schema: Option<&str>) -> PgTypeMetadata {
        let metadata_cache = self.conn.get_metadata_cache();
        let cache_key = PgMetadataCacheKey {
            schema: schema.map(Cow::Borrowed),
            type_name: Cow::Borrowed(type_name),
        };

        metadata_cache.lookup_type(&cache_key).unwrap_or_else(|| {
            let r = lookup_type(&cache_key, &self.conn);

            match r {
                Ok(type_metadata) => {
                    metadata_cache.store_type(cache_key, type_metadata);
                    type_metadata
                }
                Err(_e) => {
                    eprintln!("Failed to find oid for type `{}`", type_name);
                    // We return 0 as type oid here
                    // because most of the time postgres
                    // can figure out the type on its own
                    PgTypeMetadata {
                        oid: 0,
                        array_oid: 0,
                    }
                }
            }
        })
    }
}

fn lookup_type(
    cache_key: &PgMetadataCacheKey<'_>,
    conn: &PgConnection,
) -> QueryResult<PgTypeMetadata> {
    let search_path: String;

    let search_schema = if let Some(schema) = cache_key.schema.as_ref() {
        vec![schema.as_ref()]
    } else {
        search_path = crate::dsl::sql("SHOW search_path").get_result::<String>(conn)?;

        search_path
            .split(',')
            // skip the `$user` entry for now
            .filter(|f| !f.starts_with("\"$"))
            .map(|s| s.trim())
            .collect()
    };

    let metadata = pg_type::table
        .inner_join(pg_namespace::table)
        .filter(pg_namespace::nspname.eq(crate::dsl::any(search_schema)))
        .filter(pg_type::typname.eq(&cache_key.type_name))
        .select((pg_type::oid, pg_type::typarray))
        .first(conn)?;

    Ok(metadata)
}

#[derive(Hash, PartialEq, Eq)]
struct PgMetadataCacheKey<'a> {
    schema: Option<Cow<'a, str>>,
    type_name: Cow<'a, str>,
}

impl<'a> PgMetadataCacheKey<'a> {
    fn as_owned(&self) -> PgMetadataCacheKey<'static> {
        PgMetadataCacheKey {
            schema: self
                .schema
                .as_ref()
                .map(|schema| Cow::Owned(schema.as_ref().to_owned())),
            type_name: Cow::Owned(self.type_name.as_ref().to_owned()),
        }
    }
}

/// Stores a cache for the OID of custom types
#[allow(missing_debug_implementations)]
pub struct PgMetadataCache {
    cache: RefCell<HashMap<PgMetadataCacheKey<'static>, PgTypeMetadata>>,
}

impl PgMetadataCache {
    pub(crate) fn new() -> Self {
        PgMetadataCache {
            cache: RefCell::new(HashMap::new()),
        }
    }

    fn lookup_type(&self, type_name: &PgMetadataCacheKey) -> Option<PgTypeMetadata> {
        Some(*self.cache.borrow().get(type_name)?)
    }

    fn store_type(&self, type_name: PgMetadataCacheKey, type_metadata: PgTypeMetadata) {
        self.cache
            .borrow_mut()
            .insert(type_name.as_owned(), type_metadata);
    }
}

table! {
    pg_type (oid) {
        oid -> Oid,
        typname -> Text,
        typarray -> Oid,
        typnamespace -> Oid,
    }
}

table! {
    pg_namespace (oid) {
        oid -> Oid,
        nspname -> Text,
    }
}

joinable!(pg_type -> pg_namespace(typnamespace));
allow_tables_to_appear_in_same_query!(pg_type, pg_namespace);
