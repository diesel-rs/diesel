use super::{UpdateTarget, IncompleteUpdateStatement};
use super::delete_statement::DeleteStatement;

pub fn update<T: UpdateTarget>(source: T) -> IncompleteUpdateStatement<T> {
    IncompleteUpdateStatement::new(source)
}

pub fn delete<T: UpdateTarget>(source: T) -> DeleteStatement<T> {
    DeleteStatement::new(source)
}
