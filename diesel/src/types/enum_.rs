use core::fmt::Display;
use core::marker::PhantomData;

use crate::backend::Backend;
use crate::deserialize::{FromSql, FromSqlRef};
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::serialize::ToSql;
use crate::sql_types::EnumSqlType;

/// Metadata for an enum variant produced by `diesel-derive` in the `#[derive(Enum)]` macro
#[derive(Debug, Clone, Copy)]
pub struct EnumVariant {
    /// The discriminant value of the given enum variant
    pub discriminant: i128,
    /// The rust side name of the given enum variant
    pub rust_name: &'static str,
    /// The sql side name of the given enum variant
    pub sql_name: &'static str,
}

/// A helper trait to describe mapping an enum between rust values and database values
///
/// This is implemented for different mapping strategies, some of them might
/// be database dependent, while others might be independent
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no valid strategy to map an enum for backend `{DB}`",
    note = "the `Sqlite` backend only support mapping to `Text` and `Integer` fields"
)]
pub trait EnumMapping<DB: Backend> {
    /// Map an enum variant to the database representation
    fn map_to_database_value<'b>(
        output: &mut crate::serialize::Output<'b, '_, DB>,
        variant: &'static EnumVariant,
    ) -> crate::serialize::Result;

    /// Construct an enum variant from the database representation
    ///
    /// This is expected to return the index of the variant in the `variants` array
    fn map_from_database_value(
        raw: DB::RawValue<'_>,
        type_name: &'static str,
        variants: &'static [EnumVariant],
    ) -> crate::deserialize::Result<usize>;
}

/// Map enum variants to `TEXT` database fields by converting each variant to a string value
#[derive(Clone, Copy, Debug)]
pub struct StringMapping;

/// Map enum variants to `INTEGER` (in different sizes) by converting each variant to the relevant
/// discriminant value
///
/// The generic type `T` specifies the Rust side integer type, while the generic type `ST`
/// specifies the SQL side integer type. Both need to match
///
/// This mapping variant requires to specify explicit discriminant values for all enum variants
#[derive(Debug)]
pub struct IntMapping<T, ST>(PhantomData<(T, ST)>);

/// Map enum variants to matching database enum variants
#[derive(Clone, Copy, Debug)]
pub struct EnumTypeMapping;

impl<const ANYWAY: bool, DB> EnumSqlType<ANYWAY, DB> for crate::sql_types::Text
where
    DB: Backend,
    StringMapping: EnumMapping<DB>,
{
    type Strategy = crate::types::enum_::StringMapping;
}

impl<DB> EnumSqlType<true, DB> for crate::sql_types::Integer
where
    DB: Backend,
    crate::types::enum_::IntMapping<i32, Self>: crate::types::enum_::EnumMapping<DB>,
{
    type Strategy = crate::types::enum_::IntMapping<i32, Self>;
}

impl<DB> EnumSqlType<true, DB> for crate::sql_types::BigInt
where
    DB: Backend,
    crate::types::enum_::IntMapping<i64, Self>: crate::types::enum_::EnumMapping<DB>,
{
    type Strategy = crate::types::enum_::IntMapping<i64, Self>;
}

impl<DB> EnumSqlType<true, DB> for crate::sql_types::SmallInt
where
    DB: Backend,
    crate::types::enum_::IntMapping<i16, Self>: crate::types::enum_::EnumMapping<DB>,
{
    type Strategy = crate::types::enum_::IntMapping<i16, Self>;
}

impl<DB> EnumSqlType<true, DB> for crate::sql_types::TinyInt
where
    DB: Backend,
    IntMapping<i8, Self>: crate::types::enum_::EnumMapping<DB>,
{
    type Strategy = crate::types::enum_::IntMapping<i8, Self>;
}

impl<DB> EnumMapping<DB> for StringMapping
where
    DB: Backend,
    for<'a> &'a str: FromSqlRef<'a, crate::sql_types::Text, DB>,
    &'static str: ToSql<crate::sql_types::Text, DB>,
{
    fn map_to_database_value<'b>(
        output: &mut crate::serialize::Output<'b, '_, DB>,
        variant: &'static EnumVariant,
    ) -> crate::serialize::Result {
        <&str as crate::serialize::ToSql<crate::sql_types::Text, DB>>::to_sql(
            &variant.sql_name,
            output,
        )
    }

    fn map_from_database_value(
        mut raw: <DB as crate::backend::Backend>::RawValue<'_>,
        type_name: &'static str,
        variants: &'static [EnumVariant],
    ) -> crate::deserialize::Result<usize> {
        let s = <&str as FromSqlRef<crate::sql_types::Text, DB>>::from_sql(&mut raw)?;
        Self::from_variant_name(type_name, variants, s)
    }
}

impl StringMapping {
    #[doc(hidden)]
    pub fn from_variant_name(
        type_name: &'static str,
        variants: &'static [EnumVariant],
        s: &str,
    ) -> crate::deserialize::Result<usize> {
        variants
            .iter()
            .position(|v| v.sql_name == s)
            .ok_or_else(|| {
                alloc::format!(
                    "Invalid enum variant `{s}` for `{type_name}`. Allowed variants are {}",
                    variants
                        .iter()
                        .map(|v| alloc::format!("`{}`", v.sql_name))
                        .collect::<alloc::vec::Vec<_>>()
                        .join(", ")
                )
                .into()
            })
    }
}

impl<T, ST, DB> EnumMapping<DB> for IntMapping<T, ST>
where
    for<'a> DB: Backend<BindCollector<'a> = RawBytesBindCollector<DB>>,
    T: ToSql<ST, DB> + FromSql<ST, DB>,
    i128: TryInto<T, Error: Display> + TryFrom<T, Error: Display>,
{
    fn map_to_database_value<'b>(
        output: &mut crate::serialize::Output<'b, '_, DB>,
        variant: &'static EnumVariant,
    ) -> crate::serialize::Result {
        let v = variant.discriminant.try_into().map_err(|e| {
            alloc::format!(
                "Failed to convert discriminate to {}: {e}",
                core::any::type_name::<T>()
            )
        })?;
        <T as ToSql<ST, DB>>::to_sql(&v, &mut output.reborrow())
    }

    fn map_from_database_value(
        raw: <DB as crate::backend::Backend>::RawValue<'_>,
        type_name: &'static str,
        variants: &'static [EnumVariant],
    ) -> crate::deserialize::Result<usize> {
        let i = <T as FromSql<ST, DB>>::from_sql(raw)?;
        Self::from_discriminant(type_name, variants, i)
    }
}

impl<T, ST> IntMapping<T, ST>
where
    i128: TryFrom<T, Error: Display>,
{
    #[doc(hidden)]
    pub fn from_discriminant(
        type_name: &'static str,
        variants: &'static [EnumVariant],
        value: T,
    ) -> crate::deserialize::Result<usize> {
        let i: i128 = value.try_into().map_err(|e| {
            alloc::format!(
                "Failed to convert {} to discriminate: {e}",
                core::any::type_name::<T>()
            )
        })?;
        variants
            .iter()
            .position(|v| v.discriminant == i)
            .ok_or_else(|| {
                alloc::format!(
                    "Invalid enum variant `{i}` for `{type_name}`. Allowed variants are {}",
                    variants
                        .iter()
                        .map(|v| alloc::format!("`{}`", v.discriminant))
                        .collect::<alloc::vec::Vec<_>>()
                        .join(", ")
                )
                .into()
            })
    }
}

#[doc(hidden)]
/// Implementing this soundly requires that a reference of the Self type
/// can be created out of a i128, which is the case for signed integers with
/// smaller bit sizes
#[allow(unsafe_code)]
pub unsafe trait IntegerMappingHelper
where
    Self: TryFrom<i128, Error: Display>,
{
    fn as_ref(v: &i128) -> Result<&Self, alloc::boxed::Box<dyn core::error::Error + Send + Sync>> {
        let _: Self = (*v).try_into().map_err(|e| {
            alloc::format!(
                "Failed to convert discriminate to {}: {e}",
                core::any::type_name::<Self>()
            )
        })?;
        let v = core::slice::from_ref(v);

        let (front, v, tail) = unsafe {
            // SAFETY: This cast is sound as the trait requires that Self is a smaller
            // signed integer type than i128. A i128 consists conceptually out of multiple
            // values of a smaller integer
            v.align_to::<Self>()
        };
        debug_assert!(front.is_empty());
        debug_assert!(tail.is_empty());

        if cfg!(target_endian = "big") {
            Ok(v.last().expect("We get at least one slice element"))
        } else {
            Ok(v.first().expect("We get at least one slice element"))
        }
    }
}

// SAFETY: These impls are safe as the type sizes are smaller
// than that one of i128 and also the alignment of any
// of these types is smaller or equal to i128
#[allow(unsafe_code)]
unsafe impl IntegerMappingHelper for i64 {}
#[allow(unsafe_code)]
unsafe impl IntegerMappingHelper for i32 {}
#[allow(unsafe_code)]
unsafe impl IntegerMappingHelper for i16 {}
#[allow(unsafe_code)]
unsafe impl IntegerMappingHelper for i8 {}

#[cfg(test)]
mod tests {
    use super::IntegerMappingHelper;

    #[test]
    fn check_as_ref_i64() {
        assert_eq!(i64::as_ref(&1).unwrap(), &1);
        assert_eq!(i64::as_ref(&42).unwrap(), &42);
        assert_eq!(i64::as_ref(&0).unwrap(), &0);
        assert_eq!(i64::as_ref(&(i64::MAX as i128)).unwrap(), &i64::MAX);
        assert_eq!(i64::as_ref(&(i64::MIN as i128)).unwrap(), &i64::MIN);
        assert!(i64::as_ref(&i128::MAX).is_err());
        assert!(i64::as_ref(&i128::MIN).is_err());
        assert!(i64::as_ref(&(i64::MAX as i128 + 1)).is_err());
        assert!(i64::as_ref(&(i64::MIN as i128 - 1)).is_err());
    }

    #[test]
    fn check_as_ref_i32() {
        assert_eq!(i32::as_ref(&1).unwrap(), &1);
        assert_eq!(i32::as_ref(&42).unwrap(), &42);
        assert_eq!(i32::as_ref(&0).unwrap(), &0);
        assert_eq!(i32::as_ref(&(i32::MAX as i128)).unwrap(), &i32::MAX);
        assert_eq!(i32::as_ref(&(i32::MIN as i128)).unwrap(), &i32::MIN);
        assert!(i32::as_ref(&i128::MAX).is_err());
        assert!(i32::as_ref(&i128::MIN).is_err());
        assert!(i32::as_ref(&(i64::MAX as i128)).is_err());
        assert!(i32::as_ref(&(i64::MIN as i128)).is_err());
        assert!(i32::as_ref(&(i32::MAX as i128 + 1)).is_err());
        assert!(i32::as_ref(&(i32::MIN as i128 - 1)).is_err());
    }

    #[test]
    fn check_as_ref_i16() {
        assert_eq!(i16::as_ref(&1).unwrap(), &1);
        assert_eq!(i16::as_ref(&42).unwrap(), &42);
        assert_eq!(i16::as_ref(&0).unwrap(), &0);
        assert_eq!(i16::as_ref(&(i16::MAX as i128)).unwrap(), &i16::MAX);
        assert_eq!(i16::as_ref(&(i16::MIN as i128)).unwrap(), &i16::MIN);
        assert!(i16::as_ref(&i128::MAX).is_err());
        assert!(i16::as_ref(&i128::MIN).is_err());
        assert!(i16::as_ref(&(i32::MAX as i128)).is_err());
        assert!(i16::as_ref(&(i32::MIN as i128)).is_err());
        assert!(i16::as_ref(&(i16::MAX as i128 + 1)).is_err());
        assert!(i16::as_ref(&(i16::MIN as i128 - 1)).is_err());
    }

    #[test]
    fn check_as_ref_i8() {
        assert_eq!(i8::as_ref(&1).unwrap(), &1);
        assert_eq!(i8::as_ref(&42).unwrap(), &42);
        assert_eq!(i8::as_ref(&0).unwrap(), &0);
        assert_eq!(i8::as_ref(&(i8::MAX as i128)).unwrap(), &i8::MAX);
        assert_eq!(i8::as_ref(&(i8::MIN as i128)).unwrap(), &i8::MIN);
        assert!(i8::as_ref(&i128::MAX).is_err());
        assert!(i8::as_ref(&i128::MIN).is_err());
        assert!(i8::as_ref(&(i16::MAX as i128)).is_err());
        assert!(i8::as_ref(&(i16::MIN as i128)).is_err());
        assert!(i8::as_ref(&(i8::MAX as i128 + 1)).is_err());
        assert!(i8::as_ref(&(i8::MIN as i128 - 1)).is_err());
    }
}
