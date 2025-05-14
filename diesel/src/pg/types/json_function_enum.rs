use std::error::Error;
use std::io::Write;

use crate::pg::Pg;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::*;

#[cfg(feature = "postgres_backend")]
impl ToSql<NullValueTreatmentEnum, Pg> for NullValueTreatment {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let literal = match self {
            Self::RaiseException => "raise_exxception",
            Self::UseJsonNull => "use_json_null",
            Self::DeleteKey => "delete_key",
            Self::ReturnTarget => "return_target",
        };
        out.write_all(literal.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}
