//! Apps should not need to concern themselves with this module.
//!
//! Crates which add new types which allow numeric operators should implement these traits to
//! specify what the output is for a given right hand side
pub trait Add {
    type Rhs;
    type Output;
}

pub trait Sub {
    type Rhs;
    type Output;
}

pub trait Mul {
    type Rhs;
    type Output;
}

pub trait Div {
    type Rhs;
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

impl Add for super::Timestamp {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}

impl Sub for super::Timestamp {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}

impl Add for super::Date {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}

impl Sub for super::Date {
    type Rhs = super::Interval;
    type Output = super::Timestamp;
}
