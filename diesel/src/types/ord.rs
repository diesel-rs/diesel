use types::{self, NotNull};

pub trait SqlOrd {}

impl SqlOrd for types::SmallInt {}
impl SqlOrd for types::Integer {}
impl SqlOrd for types::BigInt {}
impl SqlOrd for types::Float {}
impl SqlOrd for types::Double {}
impl SqlOrd for types::VarChar {}
impl SqlOrd for types::Text {}
impl SqlOrd for types::Date {}
impl SqlOrd for types::Interval {}
impl SqlOrd for types::Time {}
impl SqlOrd for types::Timestamp {}
impl<T: SqlOrd + NotNull> SqlOrd for types::Nullable<T> {}
