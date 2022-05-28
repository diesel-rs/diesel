// @generated automatically by Diesel CLI.

diesel::table! {
    posts (id) {
        id -> Integer,
        title -> Varchar,
        body -> Text,
        published -> Bool,
    }
}

diesel::table! {    
    #[sql_name = "company"]
    company (CompanyID) {
        CompanyID -> Integer,
        CompanyCode -> VarChar,       
        CompanyName -> VarChar,
        CompanyNameCN -> VarChar,
        // DateCreated -> Timestamp,
        CreditAmount -> Nullable<Decimal>,
        IsHeadOffice -> Bool,
        id0 -> Integer,
            id1 -> Integer,
            id2 -> Integer,
            id3 -> Integer,
            id4 -> Integer,
            id5 -> Integer,
            id6 -> Integer,
            id7 -> Integer,
            id8 -> Integer,
            id9 -> Integer,
            id10 -> Integer,
            id11 -> Integer,
            id12 -> Integer,
            id13 -> Integer,
            id14 -> Integer,
            id15 -> Integer,
            id16 -> Integer,
            id17 -> Integer,
            id18 -> Integer,
            id19 -> Integer,
            // id20 -> Integer,
            // id21 -> Integer,
            // id22 -> Integer,
            // id23 -> Integer,
            // id24 -> Integer,
            // id25 -> Integer,
            // id26 -> Integer,
            // id27 -> Integer,
            // id28 -> Integer,
            // id29 -> Integer,            
    }
}