use std::mem;

use prelude::*;
use super::{PgConnection, PgTypeMetadata};

/// Determines the OID of types at runtime
#[allow(missing_debug_implementations)]
pub struct PgMetadataLookup {
    conn: PgConnection,
}

impl PgMetadataLookup {
    pub(crate) fn new(conn: &PgConnection) -> &Self {
        unsafe { mem::transmute(conn) }
    }

    /// Determine the type metadata for the given `type_name`
    ///
    /// This function should only be used for user defined types, or types which
    /// come from an extension. This function may perform a SQL query to look
    /// up the type. For built-in types, a static OID should be preferred.
    pub fn lookup_type(&self, type_name: &str) -> PgTypeMetadata {
        use self::pg_type::dsl::*;

        pg_type
            .select((oid, typarray))
            .filter(typname.eq(type_name))
            .first(&self.conn)
            .unwrap_or_default()
    }
}

table! {
    pg_type (oid) {
        oid -> Oid,
        typname -> Text,
        typarray -> Oid,
    }
}
