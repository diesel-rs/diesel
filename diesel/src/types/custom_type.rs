//!
//!```
//! #[macro_use]
//! extern crate diesel;
//!
//! use diesel::types::{SmallInt, CustomSqlType};
//! use std::error::Error;
//!
//! #[derive(Clone, Copy)]
//! #[repr(i16)]
//! enum Color {
//!     Red = 1,
//!     Green = 2,
//!     Blue = 3,
//! }
//!
//! // Specify how the custom type should be converted
//! impl CustomSqlType for Color {
//!     type DataBaseType = SmallInt;
//!     type RawType = i16;
//!
//!     fn to_database_type(&self) -> i16 {
//!         *self as i16
//!     }
//!
//!     fn from_database_type(v: &i16) -> Result<Self, Box<Error + Send + Sync>> {
//!         match *v {
//!             1 => Ok(Color::Red),
//!             2 => Ok(Color::Green),
//!             3 => Ok(Color::Blue),
//!             v => panic!("Unknown value {} for Color found", v),
//!         }
//!     } 
//! }
//!
//! // Add all needed implements for diesel
//! CustomSqlType!(Color);
//!
//!
//! // Use the type like every other type provided by diesel
//! table!{
//!     users{
//!         id -> Integer,
//!         name -> Text,
//!         hair_color -> Nullable<SmallInt>,
//!     }
//! }
//!
//! struct User {
//!     name: String,
//!     hair_color: Option<Color>,
//! } 
//!
//! Queryable! {
//!     struct User {
//!         name: String,
//!         hair_color: Option<Color>,
//!     }
//! }
//!
//!
//! struct NewUser<'a> {
//!     name: &'a str,
//!     hair_color: Option<Color>,
//! }
//!
//! Insertable! {
//!     (users)
//!     struct NewUser<'a> {
//!         name: &'a str,
//!         hair_color: Option<Color>,
//!     } 
//! }
//!
//!
//!# fn main(){}
//!
//!```
//!
//! Or use it with [custom_derive](https://danielkeep.github.io/rust-custom-derive/doc/custom_derive/index.html)
//!
//!```
//! #[macro_use]
//! extern crate diesel;
//! #[macro_use]
//! extern crate custom_derive;
//!
//! use diesel::types::{SmallInt, CustomSqlType};
//! use std::error::Error;
//!
//! custom_derive!{
//!     #[derive(Clone, Copy, CustomSqlType)]
//!     #[repr(i16)]
//!     enum Color {
//!         Red = 1,
//!         Green = 2,
//!         Blue = 3,
//!     }
//! }
//!
//! // Specify how the custom type should be converted
//! impl CustomSqlType for Color {
//!     type DataBaseType = SmallInt;
//!     type RawType = i16;
//!
//!     fn to_database_type(&self) -> i16 {
//!         *self as i16
//!     }
//!
//!     fn from_database_type(v: &i16) -> Result<Self, Box<Error + Send + Sync>> {
//!         match *v {
//!             1 => Ok(Color::Red),
//!             2 => Ok(Color::Green),
//!             3 => Ok(Color::Blue),
//!             v => panic!("Unknown value {} for Color found", v),
//!         }
//!     } 
//! }
//!
//! // Use the type like every other type provided by diesel
//! table!{
//!     users{
//!         id -> Integer,
//!         name -> Text,
//!         hair_color -> Nullable<SmallInt>,
//!     }
//! }
//!
//! custom_derive!{
//!     #[derive(Queryable)]
//!     struct User {
//!         name: String,
//!         hair_color: Option<Color>,
//!     }
//! }
//!
//! custom_derive!{
//!     #[derive(Insertable(users))]
//!     struct NewUser<'a> {
//!         name: &'a str,
//!         hair_color: Option<Color>,
//!     }
//! }
//!
//!
//!
//!# fn main(){}
//!
//!```

use std::error::Error;

/// Trait indicating how to convert a custom type into a diesel known SQL-type
pub trait CustomSqlType: Sized {
    /// [Diesel type](http://docs.diesel.rs/diesel/types/index.html)
    type DataBaseType;
    /// Raw rust type corresponding to the diesel type
    type RawType;

    /// How to convert the custom type into the database type
    fn to_database_type(&self) -> Self::RawType;

    /// How to convert the database type into the custom type
    fn from_database_type(&Self::RawType) -> Result<Self, Box<Error + Send + Sync>>;
}

/// Macro to generate all needed trait implementations for diesel.
/// The macro assumes that `CustomSqlType` is implemented for your target type
#[macro_export]
macro_rules! CustomSqlType {
    (() $(pub)* enum $name:ident $($tail:tt)*) => { CustomSqlType!($name); };
    (() $(pub)* struct $name:ident $($tail:tt)*) => { CustomSqlType!($name); };
    ( $Target:ident  ) => {

        impl <DB> $crate::types::ToSql<<$Target as CustomSqlType>::DataBaseType, DB> for $Target
        where $Target: CustomSqlType,
              DB: $crate::backend::Backend+ $crate::types::HasSqlType<<$Target as CustomSqlType>::DataBaseType>,
              <$Target as CustomSqlType>::RawType: $crate::types::ToSql<<$Target as CustomSqlType>::DataBaseType, DB>
        {
            fn to_sql<W: ::std::io::Write>(&self, out: &mut W) -> ::std::result::Result<$crate::types::IsNull, Box<Error + Send + Sync>>{
                <$Target as CustomSqlType>::RawType::to_sql(&Self::to_database_type(self),out)
            }
        }

        impl<DB> $crate::types::FromSql<<$Target as CustomSqlType>::DataBaseType, DB> for $Target
            where $Target: CustomSqlType,
                  DB: $crate::backend::Backend+ $crate::types::HasSqlType<<$Target as CustomSqlType>::DataBaseType>,
                  <$Target as CustomSqlType>::RawType: $crate::types::FromSql<<$Target as CustomSqlType>::DataBaseType, DB>
        {
            fn from_sql(bytes: Option<&DB::RawValue>) -> ::std::result::Result<Self, Box<Error + Send + Sync>>{
                match <$Target as CustomSqlType>::RawType::from_sql(bytes) {
                    Ok(a) => Self::from_database_type(&a),
                    Err(e) => Err(e),
                }
            }
        }

        impl<DB> $crate::types::FromSqlRow<<$Target as CustomSqlType>::DataBaseType, DB> for $Target
        where DB: $crate::backend::Backend + $crate::types::HasSqlType<<$Target as CustomSqlType>::DataBaseType>,
              $Target: $crate::types::FromSql<<$Target as CustomSqlType>::DataBaseType, DB>
        {
            fn build_from_row<R: $crate::row::Row<DB>>(row: &mut R) -> ::std::result::Result<Self, Box<Error + Send + Sync>> {
                <$Target as $crate::types::FromSql<<$Target as CustomSqlType>::DataBaseType, DB>>::from_sql(row.take())
            }
        }


        impl $crate::expression::AsExpression<<$Target as CustomSqlType>::DataBaseType> for $Target {
            type Expression = $crate::expression::bound::Bound<<$Target as CustomSqlType>::DataBaseType, $Target>;

            fn as_expression(self) -> Self::Expression {
               $crate::expression::bound::Bound::new(self)
            }
        }

        impl<'a> $crate::expression::AsExpression<<$Target as CustomSqlType>::DataBaseType> for &'a $Target {
            type Expression = $crate::expression::bound::Bound<<$Target as CustomSqlType>::DataBaseType, &'a $Target>;

            fn as_expression(self) -> Self::Expression {
                $crate::expression::bound::Bound::new(self)
            }
        }

    };
}


