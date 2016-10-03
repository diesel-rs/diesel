use diesel::*;
use diesel::pg::PgConnection;
use std::error::Error;

pub fn load_table_names(connection: &PgConnection) -> Result<Vec<String>, Box<Error>> {
    use diesel::expression::dsl::sql;

    let query = select(sql::<types::VarChar>("table_name FROM information_schema.tables"))
        .filter(sql::<types::Bool>("\
            table_schema = 'public' AND \
            table_name NOT LIKE '\\_\\_%' AND \
            table_type LIKE 'BASE TABLE'\
        "));
    Ok(try!(query.load(connection)))
}
