use types::{self, NativeSqlType};

pub trait Num {}

impl Num for types::SmallInt {}
impl Num for types::Integer {}
impl Num for types::BigInt {}
impl Num for types::Float {}
impl Num for types::Double {}
impl<T: Num + NativeSqlType> Num for types::Nullable<T> {}

