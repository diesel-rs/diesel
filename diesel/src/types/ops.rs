//! Represents the output of numeric operators in SQL
//!
//! Apps should not need to concern themselves with this module.
//! If you are looking for where the actual implementation of `std::ops::Add`
//! and friends are generated for columns, see
//! [`numeric_expr!`](../../macro.numeric_expr.html).
//!
//! Crates which add new types which allow numeric operators should implement
//! these traits to specify what the output is for a given right hand side.
//!
//! Unlike the traits in `std::ops`, the right hand side is an associated type
//! rather than a type parameter. The biggest drawback of this is that any type
//! can only have one right hand type which can be added/subtracted, etc. The
//! most immediately noticable effect of this is that you cannot add a nullable
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

/// Represents SQL types which can be added.
pub trait Add {
    /// The SQL type which can be added to this one
    type Rhs;
    /// The SQL type of the result of adding `Rhs` to `Self`
    type Output;
}

/// Represents SQL types which can be subtracted.
pub trait Sub {
    /// The SQL type which can be subtracted from this one
    type Rhs;
    /// The SQL type of the result of subtracting `Rhs` from `Self`
    type Output;
}

/// Represents SQL types which can be multiplied.
pub trait Mul {
    /// The SQL type which this can be multiplied by
    type Rhs;
    /// The SQL type of the result of multiplying `Self` by `Rhs`
    type Output;
}

/// Represents SQL types which can be divided.
pub trait Div {
    /// The SQL type which this one can be divided by
    type Rhs;
    /// The SQL type of the result of dividing `Self` by `Rhs`
    type Output;
}

macro_rules! numeric_type {
    ($($tpe: ident),*) => {
        $(
        impl Add for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
        }

        impl Add for super::Nullable<super::$tpe> {
            type Rhs = super::Nullable<super::$tpe>;
            type Output = super::Nullable<super::$tpe>;
        }

        impl Sub for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
        }

        impl Sub for super::Nullable<super::$tpe> {
            type Rhs = super::Nullable<super::$tpe>;
            type Output = super::Nullable<super::$tpe>;
        }

        impl Mul for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
        }

        impl Mul for super::Nullable<super::$tpe> {
            type Rhs = super::Nullable<super::$tpe>;
            type Output = super::Nullable<super::$tpe>;
        }

        impl Div for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
        }

        impl Div for super::Nullable<super::$tpe> {
            type Rhs = super::Nullable<super::$tpe>;
            type Output = super::Nullable<super::$tpe>;
        }
        )*
    }
}

numeric_type!(SmallInt, Integer, BigInt, Float, Double, Numeric);

impl Add for super::Time {
    type Rhs = super::Interval;
    type Output = super::Time;
}

impl Add for super::Nullable<super::Time> {
    type Rhs = super::Nullable<super::Interval>;
    type Output = super::Nullable<super::Time>;
}

impl Sub for super::Time {
    type Rhs = super::Interval;
    type Output = super::Time;
}

impl Sub for super::Nullable<super::Time> {
    type Rhs = super::Nullable<super::Interval>;
    type Output = super::Nullable<super::Time>;
}

impl Add for super::Date {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}

impl Add for super::Nullable<super::Date> {
    type Rhs = super::Nullable<super::Interval>;
    type Output = super::Nullable<super::Timestamp>;
}

impl Sub for super::Date {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}

impl Sub for super::Nullable<super::Date> {
    type Rhs = super::Nullable<super::Interval>;
    type Output = super::Nullable<super::Timestamp>;
}

impl Add for super::Timestamp {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}

impl Add for super::Nullable<super::Timestamp> {
    type Rhs = super::Nullable<super::Interval>;
    type Output = super::Nullable<super::Timestamp>;
}

impl Sub for super::Timestamp {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}

impl Sub for super::Nullable<super::Timestamp> {
    type Rhs = super::Nullable<super::Interval>;
    type Output = super::Nullable<super::Timestamp>;
}
