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
            Err(_e) => {
                let metadata_cache = self.get_metadata_cache();
                let lower_schema = schema.map(|schema| schema.to_lowercase()).unwrap();
                if let Some(_) = metadata_cache.lookup_type(&PgMetadataCacheKey {
                    schema: Some(Cow::Borrowed(lower_schema.as_str())),
                    type_name: Cow::Borrowed(type_name.to_lowercase().as_str()),
                }) {
                    return PgTypeMetadata(Err(FailedToLookupTypeError::InvalidCasing(Box::new(
                        cache_key.into_owned(),
                    ))));
                }
                PgTypeMetadata(Err(FailedToLookupTypeError::TypeNotFound(Box::new(
                    cache_key.into_owned(),
                ))))
            }
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
    let metadata_query = pg_type::table.select((pg_type::oid, pg_type::typarray));

    let metadata = if let Some(schema) = cache_key.schema.as_deref() {
        // We have an explicit type name and schema given here
        // that should resolve to an unique type
        metadata_query
            .inner_join(pg_namespace::table)
            .filter(pg_type::typname.eq(&cache_key.type_name))
            .filter(pg_namespace::nspname.eq(schema))
            .first(conn)?
    } else {
        // We don't have a schema here. A type with the same name could exists
        // in more than one schema at once, even in more than one schema that
        // is in the current search path. As we don't want to reimplement
        // postgres name resolution here we just cast the time name to a concrete type
        // and let postgres figure out the name resolution on it's own.
        metadata_query
            .filter(
                pg_type::oid.eq(crate::dsl::sql("")
                    .bind::<crate::sql_types::Text, _>(&cache_key.type_name)
                    .sql("::regtype::oid")),
            )
            .first(conn)?
    };

    Ok(metadata)
}

/// The key used to lookup cached type oid's inside of
/// a [PgMetadataCache].
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct PgMetadataCacheKey<'a> {
    pub(in crate::pg) schema: Option<Cow<'a, str>>,
    pub(in crate::pg) type_name: Cow<'a, str>,
}

impl<'a> PgMetadataCacheKey<'a> {
    /// Construct a new cache key from an optional schema name and
    /// a type name
    pub fn new(schema: Option<Cow<'a, str>>, type_name: Cow<'a, str>) -> Self {
        Self { schema, type_name }
    }

    /// Convert the possibly borrowed version of this metadata cache key
    /// into a lifetime independ owned version
    pub fn into_owned(self) -> PgMetadataCacheKey<'static> {
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
    pub fn lookup_type(&self, type_name: &PgMetadataCacheKey) -> Option<PgTypeMetadata> {
        Some(PgTypeMetadata(Ok(*self.cache.get(type_name)?)))
    }

    /// Store the OID of a custom type
    pub fn store_type(
        &mut self,
        type_name: PgMetadataCacheKey,
        type_metadata: impl Into<InnerPgTypeMetadata>,
    ) {
        self.cache
            .insert(type_name.into_owned(), type_metadata.into());
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
