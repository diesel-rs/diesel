//! Everything related to table aliasing
//!
//! See [`alias!`](crate::alias!) for more details

mod alias;
mod aliased_field;
mod dsl_impls;
mod field_alias_mapper;
mod joins;
mod macros;

pub use alias::{Alias, AliasAliasAppearsInFromClause, AliasAppearsInFromClause};
pub use aliased_field::AliasedField;
pub use field_alias_mapper::{FieldAliasMapper, FieldAliasMapperAssociatedTypesDisjointnessTrick};

/// Types created by the `alias!` macro that serve to distinguish between aliases implement
/// this trait.
///
/// In order to be able to implement within diesel a lot of traits on what will represent the alias,
/// we cannot use directly that new type within the query builder. Instead, we will use `Alias<S>`,
/// where `S: AliasSource`.
///
/// This trait should never be implemented by an end-user directly.
pub trait AliasSource {
    /// The name of this alias in the query
    const NAME: &'static str;
    /// The table the alias maps to
    type Target;
    /// Obtain the table from the source
    ///
    /// (used by Diesel to implement some traits)
    fn target(&self) -> &Self::Target;
}
