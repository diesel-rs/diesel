diesel::table! {
    posts (id) {
        id -> Integer,
        title -> Varchar,
        body -> Text,
        published -> Bool,
    }
}

diesel::table! {    
    company (CompanyID) {
        CompanyID -> Integer,
        CompanyCode -> VarChar,       
        CompanyName -> VarChar,
        CompanyNameCN -> VarChar,
        DateCreated -> Timestamp,
        // CreditAmount -> Double,
        // IsHeadOffice -> Bool,
    }
}