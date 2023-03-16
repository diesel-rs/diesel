use crate::schema::connection;
use diesel::pg::Pg;
use diesel::query_builder::Query;
use diesel::query_builder::{AstPass, QueryFragment, QueryId};
use diesel::sql_types::Text;
use diesel::{QueryResult, RunQueryDsl};

#[derive(Debug, Clone)]
pub struct LiteralSelect<'a> {
    pub(crate) table_name: &'a str,
    pub(crate) literal: String,
}

impl<'a> QueryFragment<Pg> for LiteralSelect<'a> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        out.push_sql("select ");
        out.push_bind_param::<Text, _>(self.literal.as_str())?;
        out.push_sql("  from ");
        out.push_sql(self.table_name);

        Ok(())
    }
}

impl<'a> QueryId for LiteralSelect<'a> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, Conn> RunQueryDsl<Conn> for LiteralSelect<'a> {}

impl<'a> Query for LiteralSelect<'a> {
    type SqlType = Text;
}

#[test]
fn literal_select_using_query_fragment() {
    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(connection)
        .unwrap();

    let expected_data = vec!["name".to_string(), "name".to_string()];

    let query = LiteralSelect {
        table_name: "users",
        literal: "name".to_string(),
    };
    let actual_data: Vec<_> = query.load::<String>(connection).unwrap();
    assert_eq!(expected_data, actual_data);
}
