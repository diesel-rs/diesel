//! Apps should not need to concern themselves with this module.
//!
//! Crates which add new types which allow numeric operators should implement these traits to
//! specify what the output is for a given right hand side

/// Represents SQL types which can be added.
///
/// Similar to `std::ops::Add`, but this only includes information about the SQL types that will
/// result from the operation. Unlike `std::ops::Add`, the right side is an associated type rather
/// than a type parameter. This means that a given SQL type can only have one other SQL type added
/// to it. The reason for this is that when the right side is a Rust value which would be sent as a
/// bind parameter, we need to know which type to use.
pub trait Add {
    /// The SQL type which can be added to this one
    type Rhs;
    /// The SQL type of the result of adding `Rhs` to `Self`
    type Output;
}

/// Represents SQL types which can be subtracted.
///
/// Similar to `std::ops::Sub`, but this only includes information about the SQL types that will
/// result from the operation. Unlike `std::ops::Sub`, the right side is an associated type rather
/// than a type parameter. This means that a given SQL type can only have one other SQL type
/// subtracted from it. The reason for this is that when the right side is a Rust value which would
/// be sent as a bind parameter, we need to know which type to use.
pub trait Sub {
    /// The SQL type which can be subtracted from this one
    type Rhs;
    /// The SQL type of the result of subtracting `Rhs` from `Self`
    type Output;
}

/// Represents SQL types which can be multiplied.
///
/// Similar to `std::ops::Mul`, but this only includes information about the SQL types that will
/// result from the operation. Unlike `std::ops::Mul`, the right side is an associated type rather
/// than a type parameter. This means that a given SQL type can only have one other SQL type
/// multiplied with it. The reason for this is that when the right side is a Rust value which
/// would be sent as a bind parameter, we need to know which type to use.
pub trait Mul {
    /// The SQL type which this can be multiplied by
    type Rhs;
    /// The SQL type of the result of multiplying `Self` by `Rhs`
    type Output;
}

/// Represents SQL types which can be divided.
///
/// Similar to `std::ops::Div`, but this only includes information about the SQL types that will
/// result from the operation. Unlike `std::ops::Div`, the right side is an associated type rather
/// than a type parameter. This means that a given SQL type can only be divided by one other SQL
/// type. The reason for this is that when the right side is a Rust value which would be sent as a
/// bind parameter, we need to know which type to use.
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

macro_rules! interval_type {
    ($($tpe: ident),*) => {
        $(
        impl Add for super::$tpe {
            type Rhs = super::Interval;
            type Output = super::$tpe;
        }

        impl Add for super::Nullable<super::$tpe> {
            type Rhs = super::Nullable<super::Interval>;
            type Output = super::Nullable<super::$tpe>;
        }

        impl Sub for super::$tpe {
            type Rhs = super::Interval;
            type Output = super::$tpe;
        }

        impl Sub for super::Nullable<super::$tpe> {
            type Rhs = super::Nullable<super::Interval>;
            type Output = super::Nullable<super::$tpe>;
        }
        )*
    }
}

interval_type!(Date, Timestamp);
