//! A simple implementation of peano numbers.
//!
//! This is used to enforce that columns can only be selected from a given from
//! clause if their table appears exactly one time.

/// A table never appears in the from clause.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Never;

/// A table appears in the from clause exactly one time.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Once;

/// A table appears in the from clause two or more times.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct MoreThanOnce;

/// Add two peano numbers together.
///
/// This is used to determine the number of times a table appears in a from
/// clause when the from clause contains a join.
pub trait Plus<T> {
    /// The result of adding these numbers together
    type Output;
}

impl<T> Plus<T> for Never {
    type Output = T;
}

impl<T> Plus<T> for MoreThanOnce {
    type Output = Self;
}

impl Plus<Never> for Once {
    type Output = Self;
}

impl Plus<Once> for Once {
    type Output = MoreThanOnce;
}

impl Plus<MoreThanOnce> for Once {
    type Output = MoreThanOnce;
}
