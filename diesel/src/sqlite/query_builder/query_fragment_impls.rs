use crate::query_builder::select_statement::boxed::BoxedQueryHelper;
use crate::query_builder::upsert::into_conflict_clause::OnConflictSelectWrapper;
use crate::query_builder::where_clause::BoxedWhereClause;
use crate::query_builder::where_clause::WhereClause;
use crate::query_builder::AstPass;
use crate::query_builder::BoxedSelectStatement;
use crate::query_builder::QueryFragment;
use crate::query_builder::SelectStatement;
use crate::QueryResult;

// The corresponding impl for`NoWhereClause` is missing because of
// https://www.sqlite.org/lang_UPSERT.html (Parsing Ambiguity)
#[cfg(feature = "sqlite")]
impl<F, S, D, W, O, LOf, G, H, LC> QueryFragment<crate::sqlite::Sqlite>
    for OnConflictSelectWrapper<SelectStatement<F, S, D, WhereClause<W>, O, LOf, G, H, LC>>
where
    SelectStatement<F, S, D, WhereClause<W>, O, LOf, G, H, LC>:
        QueryFragment<crate::sqlite::Sqlite>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()> {
        self.0.walk_ast(out)
    }
}

#[cfg(feature = "sqlite")]
impl<'a, ST, QS, GB> QueryFragment<crate::sqlite::Sqlite>
    for OnConflictSelectWrapper<BoxedSelectStatement<'a, ST, QS, crate::sqlite::Sqlite, GB>>
where
    BoxedSelectStatement<'a, ST, QS, crate::sqlite::Sqlite, GB>:
        QueryFragment<crate::sqlite::Sqlite>,
    QS: QueryFragment<crate::sqlite::Sqlite>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()> {
        // https://www.sqlite.org/lang_UPSERT.html (Parsing Ambiguity)
        self.0.build_query(pass, |where_clause, mut pass| {
            match where_clause {
                BoxedWhereClause::None => pass.push_sql(" WHERE 1=1 "),
                w => w.walk_ast(pass.reborrow())?,
            }
            Ok(())
        })
    }
}
