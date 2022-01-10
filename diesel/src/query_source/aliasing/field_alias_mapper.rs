use super::{Alias, AliasSource, AliasedField};

use crate::expression;
use crate::query_source::{Column, Table, TableNotEqual};

/// Serves to map `Self` to `Alias<S>`
///
/// Any column `Self` that belongs to `S::Table` will be transformed into `AliasedField<S, Self>`
///
/// Any column `Self` that does not belong to `S::Table` will be left untouched.
///
/// This also works with tuples and some expressions.
///
// This first part is implemented by the `table!` macro.
// The second part is useful to implement the joins, and may be useful to an end-user for
// ergonomics.
pub trait FieldAliasMapper<S> {
    /// Output type when mapping `C` to `Alias<S>`
    ///
    /// If `C: Column<Table = S::Table>`, `Out = AliasedField<S, C>`  
    /// Otherwise, `Out = C`
    type Out;

    /// Does the mapping
    fn map(self, alias: &Alias<S>) -> Self::Out;
}

#[doc(hidden)]
/// Allows implementing `FieldAliasMapper` in external crates without running into conflicting impl
/// errors due to https://github.com/rust-lang/rust/issues/20400
///
/// We will always have `Self = S::Table` and `CT = C::Table`
pub trait FieldAliasMapperAssociatedTypesDisjointnessTrick<CT, S, C> {
    type Out;
    fn map(column: C, alias: &Alias<S>) -> Self::Out;
}
impl<S, C> FieldAliasMapper<S> for C
where
    S: AliasSource,
    C: Column,
    S::Target: FieldAliasMapperAssociatedTypesDisjointnessTrick<C::Table, S, C>,
{
    type Out = <S::Target as FieldAliasMapperAssociatedTypesDisjointnessTrick<C::Table, S, C>>::Out;

    fn map(self, alias: &Alias<S>) -> Self::Out {
        <S::Target as FieldAliasMapperAssociatedTypesDisjointnessTrick<C::Table, S, C>>::map(
            self, alias,
        )
    }
}

impl<TS, TC, S, C> FieldAliasMapperAssociatedTypesDisjointnessTrick<TC, S, C> for TS
where
    S: AliasSource<Target = TS>,
    C: Column<Table = TC>,
    TC: Table,
    TS: TableNotEqual<TC>,
{
    type Out = C;

    fn map(column: C, _alias: &Alias<S>) -> Self::Out {
        // left untouched because the tables are different
        column
    }
}

macro_rules! field_alias_mapper {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<_S, $($T,)*> FieldAliasMapper<_S> for ($($T,)*)
            where
                _S: AliasSource,
                $($T: FieldAliasMapper<_S>,)*
            {
                type Out = ($(<$T as FieldAliasMapper<_S>>::Out,)*);

                fn map(self, alias: &Alias<_S>) -> Self::Out {
                    (
                        $(self.$idx.map(alias),)*
                    )
                }
            }
        )*
    }
}

diesel_derives::__diesel_for_each_tuple!(field_alias_mapper);

// The following `FieldAliasMapper` impls are useful for the generic join implementations.
// More may be added.
impl<SPrev, SNew, F> FieldAliasMapper<SNew> for AliasedField<SPrev, F>
where
    SNew: AliasSource,
{
    type Out = Self;

    fn map(self, _alias: &Alias<SNew>) -> Self::Out {
        // left untouched because it has already been aliased
        self
    }
}

impl<S, F> FieldAliasMapper<S> for expression::nullable::Nullable<F>
where
    F: FieldAliasMapper<S>,
{
    type Out = expression::nullable::Nullable<<F as FieldAliasMapper<S>>::Out>;

    fn map(self, alias: &Alias<S>) -> Self::Out {
        expression::nullable::Nullable::new(self.0.map(alias))
    }
}

impl<S, F> FieldAliasMapper<S> for expression::grouped::Grouped<F>
where
    F: FieldAliasMapper<S>,
{
    type Out = expression::grouped::Grouped<<F as FieldAliasMapper<S>>::Out>;

    fn map(self, alias: &Alias<S>) -> Self::Out {
        expression::grouped::Grouped(self.0.map(alias))
    }
}
