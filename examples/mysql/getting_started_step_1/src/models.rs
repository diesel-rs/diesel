use diesel::prelude::*;
use chrono::NaiveDateTime;

#[derive(Queryable)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Queryable)]
pub struct Company {
    pub CompanyID: i32,
    pub CompanyCode: String,
    pub CompanyName: String,
    pub CompanyNameCN: String,
    // pub DateCreated: NaiveDateTime,
    pub CreditAmount: f64,
    pub IsHeadOffice: bool,
    id0 : i32,
    id1 : i32,
    id2 : i32,
    id3 : i32,
    id4 : i32,
    id5 : i32,
    id6 : i32,
    id7 : i32,
    id8 : i32,
    id9 : i32,
    id10 : i32,
    id11 : i32,
    id12 : i32,
    id13 : i32,
    id14 : i32,
    id15 : i32,
    id16 : i32,
    id17 : i32,
    id18 : i32,
    id19 : i32,
    // id20 : i32,
    // id21 : i32,
    // id22 : i32,
    // id23 : i32,
    // id24 : i32,
    // id25 : i32,
    // id26 : i32,
    // id27 : i32,
    // id28 : i32,
    // id29 : i32,
}