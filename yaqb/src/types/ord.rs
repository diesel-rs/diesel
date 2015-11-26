use types::{self, NativeSqlType};

pub trait SqlOrd {}

impl SqlOrd for types::SmallInt {}
impl SqlOrd for types::Integer {}
impl SqlOrd for types::BigInt {}
impl SqlOrd for types::Float {}
impl SqlOrd for types::Double {}
impl SqlOrd for types::VarChar {}
impl SqlOrd for types::Text {}
impl<T: SqlOrd + NativeSqlType> SqlOrd for types::Nullable<T> {}
