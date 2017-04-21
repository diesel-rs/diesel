mod on_conflict_actions;
mod on_conflict_clause;
mod on_conflict_extension;
mod on_conflict_target;

pub use self::on_conflict_actions::{do_nothing, do_update, excluded};
pub use self::on_conflict_extension::OnConflictExtension;
pub use self::on_conflict_target::on_constraint;
