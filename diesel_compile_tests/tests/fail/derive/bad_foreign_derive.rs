extern crate diesel;

use diesel::deserialize::FromSqlRow;
use diesel::expression::{AsExpression, ValidGrouping};

#[derive(ValidGrouping)]
#[diesel(foreign_derive = true)]
struct User1;

#[derive(ValidGrouping)]
#[diesel(foreign_derive)]
struct User2;

#[derive(AsExpression)]
#[diesel(sql_type = bool)]
#[diesel(foreign_derive)]
struct User3;

#[derive(FromSqlRow)]
#[diesel(foreign_derive)]
struct User4;

fn main() {}
