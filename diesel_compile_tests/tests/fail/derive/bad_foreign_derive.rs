extern crate diesel;

use diesel::deserialize::FromSqlRow;
use diesel::expression::{AsExpression, ValidGrouping};

#[derive(ValidGrouping)]
#[diesel(foreign_derive = true)]
//~^ ERROR: expected `,`
struct User1;

#[derive(ValidGrouping)]
//~^ ERROR: foreign_derive requires at least one field
#[diesel(foreign_derive)]
struct User2;

#[derive(AsExpression)]
//~^ ERROR: foreign_derive requires at least one field
#[diesel(sql_type = bool)]
#[diesel(foreign_derive)]
struct User3;

#[derive(FromSqlRow)]
//~^ ERROR: foreign_derive requires at least one field
#[diesel(foreign_derive)]
struct User4;

fn main() {}
