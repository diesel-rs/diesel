use types::{self, NativeSqlType};

pub trait Num {
  type Output: NativeSqlType;
}

impl Num for types::SmallInt {
  type Output = types::BigInt;
}
impl Num for types::Integer {
  type Output = types::BigInt;
}
impl Num for types::BigInt {
  type Output = types::BigInt;
}
impl Num for types::Float {
  type Output = types::Float;
}
impl Num for types::Double {
  type Output = types::Double;
}
// impl<T: Num + NativeSqlType> Num for types::Nullable<T> {
//   type Output = NativeSqlType;
// }

