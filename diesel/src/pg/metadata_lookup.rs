#![allow(unused_parens)] // FIXME: Remove this attribute once false positive is resolved.

use super::backend::{FailedToLookupTypeError, InnerPgTypeMetadata};
use super::{Pg, PgTypeMetadata};
use crate::prelude::*;

use std::borrow::Cow;
use std::collections::HashMap;

/// Determines the OID of types at runtime
///
/// Custom implementations of `Connection<Backend = Pg>` should not implement this trait directly.
/// Instead `GetPgMetadataCache` should be implemented, afterwards the generic implementation will provide
/// the necessary functions to perform the type lookup.
pub trait PgMetadataLookup {
    /// Determine the type metadata for the given `type_name`
    ///
    /// This function should only be used for user defined types, or types which
    /// come from an extension. This function may perform a SQL query to look
    /// up the type. For built-in types, a static OID should be preferred.
    fn lookup_type(&mut self, type_name: &str, schema: Option<&str>) -> PgTypeMetadata;
}

impl<T> PgMetadataLookup for T
where
    T: Connection<Backend = Pg> + GetPgMetadataCache,
{
    fn lookup_type(&mut self, type_name: &str, schema: Option<&str>) -> PgTypeMetadata {
        let cache_key = PgMetadataCacheKey {
            schema: schema.map(Cow::Borrowed),
            type_name: Cow::Borrowed(type_name),
        };

        {
            let metadata_cache = self.get_metadata_cache();

            if let Some(metadata) = metadata_cache.lookup_type(&cache_key) {
                return metadata;
            }
        }

        let r = lookup_type(&&cache_key, self);

        match r {
            Ok(type_metadata) => {
                self.get_metadata_cache()
                    .store_type(cache_key, type_metadata);
                PgTypeMetadata(Ok(type_metadata))
            }
            Err(_e) => PgTypeMetadata(Err(FailedToLookupTypeError::new(cache_key.into_owned()))),
        }
    }
}

/// Gets the `PgMetadataCache` for a `Connection<Backend=Pg>`
/// so that the lookup of user defined types, or types which come from an extension can be cached.
///
/// Implementing this trait for a `Connection<Backend=Pg>` will cause `PgMetadataLookup` to be auto implemented.
pub trait GetPgMetadataCache {
    /// Get the `PgMetadataCache`
    fn get_metadata_cache(&mut self) -> &mut PgMetadataCache;
}

fn lookup_type<T: Connection<Backend = Pg>>(
    cache_key: &PgMetadataCacheKey<'_>,
    conn: &mut T,
) -> QueryResult<InnerPgTypeMetadata> {
    let search_path: String;
    let mut search_path_has_temp_schema = false;

    let search_schema = match cache_key.schema.as_deref() {
        Some("pg_temp") => {
            search_path_has_temp_schema = true;
            Vec::new()
        }
        Some(schema) => vec![schema],
        None => {
            search_path = crate::dsl::sql::<crate::sql_types::Text>("SHOW search_path")
                .get_result::<String>(conn)?;

            search_path
                .split(',')
                // skip the `$user` entry for now
                .filter(|f| !f.starts_with("\"$"))
                .map(|s| s.trim())
                .filter(|&f| {
                    let is_temp = f == "pg_temp";
                    search_path_has_temp_schema |= is_temp;
                    !is_temp
                })
                .collect()
        }
    };

    let metadata_query = pg_type::table
        .inner_join(pg_namespace::table)
        .filter(pg_type::typname.eq(&cache_key.type_name))
        .select((pg_type::oid, pg_type::typarray));
    let nspname_filter = pg_namespace::nspname.eq(crate::dsl::any(search_schema));

    let metadata = if search_path_has_temp_schema {
        metadata_query
            .filter(nspname_filter.or(pg_namespace::oid.eq(pg_my_temp_schema())))
            .first(conn)?
    } else {
        metadata_query.filter(nspname_filter).first(conn)?
    };

    Ok(metadata)
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub(in crate::pg) struct PgMetadataCacheKey<'a> {
    pub(crate) schema: Option<Cow<'a, str>>,
    pub(crate) type_name: Cow<'a, str>,
}

impl<'a> PgMetadataCacheKey<'a> {
    fn into_owned(self) -> PgMetadataCacheKey<'static> {
        let PgMetadataCacheKey { schema, type_name } = self;
        PgMetadataCacheKey {
            schema: schema.map(|s| Cow::Owned(s.into_owned())),
            type_name: Cow::Owned(type_name.into_owned()),
        }
    }
}

/// Cache for the [OIDs] of custom Postgres types
///
/// [OIDs]: https://www.postgresql.org/docs/current/static/datatype-oid.html
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct PgMetadataCache {
    cache: HashMap<PgMetadataCacheKey<'static>, InnerPgTypeMetadata>,
}

impl PgMetadataCache {
    /// Construct a new `PgMetadataCache`
    pub fn new() -> Self {
        Default::default()
    }

    /// Lookup the OID of a custom type
    fn lookup_type(&self, type_name: &PgMetadataCacheKey) -> Option<PgTypeMetadata> {
        Some(PgTypeMetadata(Ok(*self.cache.get(type_name)?)))
    }

    /// Store the OID of a custom type
    fn store_type(&mut self, type_name: PgMetadataCacheKey, type_metadata: InnerPgTypeMetadata) {
        self.cache.insert(type_name.into_owned(), type_metadata);
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

sql_function! { fn pg_my_temp_schema() -> Oid; }
