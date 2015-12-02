//! Apps should not need to concern themselves with this module.
//!
//! Crates which add new types which allow numeric operators should implement these traits to
//! specify what the output is for a given right hand side
use super::NativeSqlType;

pub trait Add: NativeSqlType {
    type Rhs: NativeSqlType;
    type Output: NativeSqlType;
}

pub trait Sub: NativeSqlType {
    type Rhs: NativeSqlType;
    type Output: NativeSqlType;
}

pub trait Mul: NativeSqlType {
    type Rhs: NativeSqlType;
    type Output: NativeSqlType;
}

pub trait Div: NativeSqlType {
    type Rhs: NativeSqlType;
    type Output: NativeSqlType;
}

macro_rules! numeric_type {
    ($($tpe: ident),*) => {
        $(
        impl Add for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
        }

        impl Sub for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
        }

        impl Mul for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
        }

        impl Div for super::$tpe {
            type Rhs = super::$tpe;
            type Output = super::$tpe;
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
