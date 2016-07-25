mod belongs_to;

use std::hash::Hash;

use query_dsl::FindDsl;
use query_source::Table;

pub use self::belongs_to::{BelongsTo, GroupedBy};

pub trait Identifiable {
    type Id: Hash + Eq + Copy;
    type Table: Table + FindDsl<Self::Id>;

    fn table() -> Self::Table;
    fn id(&self) -> Self::Id;
}
