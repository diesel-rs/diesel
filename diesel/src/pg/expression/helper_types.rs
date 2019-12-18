use crate::dsl::AsExprOf;
use crate::sql_types::VarChar;

/// The return type of `lhs.ilike(rhs)`
pub type ILike<Lhs, Rhs> = super::operators::ILike<Lhs, AsExprOf<Rhs, VarChar>>;

/// The return type of `lhs.not_ilike(rhs)`
pub type NotILike<Lhs, Rhs> = super::operators::NotILike<Lhs, AsExprOf<Rhs, VarChar>>;
