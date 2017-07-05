use std::mem;

use prelude::*;
use super::{PgConnection, PgTypeMetadata};

#[allow(missing_debug_implementations)]
pub struct PgMetadataLookup {
    conn: PgConnection,
}

impl PgMetadataLookup {
    pub(crate) fn new(conn: &PgConnection) -> &Self {
        unsafe { mem::transmute(conn) }
    }

    pub fn lookup_type(&self, type_name: &str) -> PgTypeMetadata {
        use self::pg_type::dsl::*;

        pg_type.select((oid, typarray))
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
