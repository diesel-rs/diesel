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
    pub DateCreated: NaiveDateTime,
    // pub CreditAmount: f64,
    // pub IsHeadOffice: bool,
}