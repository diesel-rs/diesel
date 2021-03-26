//! Represents the output of numeric operators in SQL
//!
//! Apps should not need to concern themselves with this module.
//! If you are looking for where the actual implementation of `std::ops::Add`
//! and friends are generated for columns, see
//! [`numeric_expr!`](super::super::numeric_expr!).
//!
//! Crates which add new types which allow numeric operators should implement
//! these traits to specify what the output is for a given right hand side.
//!
//! Unlike the traits in `std::ops`, the right hand side is an associated type
//! rather than a type parameter. The biggest drawback of this is that any type
//! can only have one right hand type which can be added/subtracted, etc. The
//! most immediately noticeable effect of this is that you cannot add a nullable
//! number to one that is not nullable.
//!
//! The reason for this is because of the impl of `std::ops::Add` that we need
//! to be able to write. We want the right hand side to allow Rust values which
//! should be sent as bind parameters, not just other Diesel expressions. That
//! means the impl would look like this:
//!
//! ```ignore
//! impl<ST, T> std::ops::Add<T> for my_column
//! where
//!     T: AsExpression<ST>,
//!     my_column::SqlType: diesel::ops::Add<ST>,
//! ```
//!
//! This impl is not valid in Rust, as `ST` is not constrained by the trait or
//! the implementing type. If there were two valid types for `ST` which
//! satisfied all constraints, Rust would not know which one to use, and there
//! would be no way for the user to specify which one should be used.

use super::*;

/// Represents SQL types which can be added.
pub trait Add {
    /// The SQL type which can be added to this one
    type Rhs: SqlType;
    /// The SQL type of the result of adding `Rhs` to `Self`
    type Output: SqlType;
}

/// Represents SQL types which can be subtracted.
pub trait Sub {
    /// The SQL type which can be subtracted from this one
    type Rhs: SqlType;
    /// The SQL type of the result of subtracting `Rhs` from `Self`
    type Output: SqlType;
}

/// Represents SQL types which can be multiplied.
pub trait Mul {
    /// The SQL type which this can be multiplied by
    type Rhs: SqlType;
    /// The SQL type of the result of multiplying `Self` by `Rhs`
    type Output: SqlType;
}

/// Represents SQL types which can be divided.
pub trait Div {
    /// The SQL type which this one can be divided by
    type Rhs: SqlType;
    /// The SQL type of the result of dividing `Self` by `Rhs`
    type Output: SqlType;
}

macro_rules! numeric_type {
    ($($tpe: ident),*) => {
        $(
        impl Add for $tpe {
            type Rhs = $tpe;
            type Output = $tpe;
        }

        impl Sub for $tpe {
            type Rhs = $tpe;
            type Output = $tpe;
        }

        impl Mul for $tpe {
            type Rhs = $tpe;
            type Output = $tpe;
        }

        impl Div for $tpe {
            type Rhs = $tpe;
            type Output = $tpe;
        }
        )*
    }
}

numeric_type!(SmallInt, Integer, BigInt, Float, Double, Numeric);

impl Add for Time {
    type Rhs = Interval;
    type Output = Time;
}

impl Sub for Time {
    type Rhs = Interval;
    type Output = Time;
}

impl Add for Date {
    type Rhs = Interval;
    type Output = Timestamp;
}

impl Sub for Date {
    type Rhs = Interval;
    type Output = Timestamp;
}

impl Add for Timestamp {
    type Rhs = Interval;
    type Output = Timestamp;
}

impl Sub for Timestamp {
    type Rhs = Interval;
    type Output = Timestamp;
}

impl Add for Interval {
    type Rhs = Interval;
    type Output = Interval;
}

impl Sub for Interval {
    type Rhs = Interval;
    type Output = Interval;
}

impl Mul for Interval {
    type Rhs = Integer;
    type Output = Interval;
}

impl Div for Interval {
    type Rhs = Integer;
    type Output = Interval;
}

impl<T> Add for Nullable<T>
where
    T: Add + SqlType<IsNull = is_nullable::NotNull>,
    T::Rhs: SqlType<IsNull = is_nullable::NotNull>,
    T::Output: SqlType<IsNull = is_nullable::NotNull>,
{
    type Rhs = Nullable<T::Rhs>;
    type Output = Nullable<T::Output>;
}

impl<T> Sub for Nullable<T>
where
    T: Sub + SqlType<IsNull = is_nullable::NotNull>,
    T::Rhs: SqlType<IsNull = is_nullable::NotNull>,
    T::Output: SqlType<IsNull = is_nullable::NotNull>,
{
    type Rhs = Nullable<T::Rhs>;
    type Output = Nullable<T::Output>;
}

impl<T> Mul for Nullable<T>
where
    T: Mul + SqlType<IsNull = is_nullable::NotNull>,
    T::Rhs: SqlType<IsNull = is_nullable::NotNull>,
    T::Output: SqlType<IsNull = is_nullable::NotNull>,
{
    type Rhs = Nullable<T::Rhs>;
    type Output = Nullable<T::Output>;
}

impl<T> Div for Nullable<T>
where
    T: Div + SqlType<IsNull = is_nullable::NotNull>,
    T::Rhs: SqlType<IsNull = is_nullable::NotNull>,
    T::Output: SqlType<IsNull = is_nullable::NotNull>,
{
    type Rhs = Nullable<T::Rhs>;
    type Output = Nullable<T::Output>;
}
