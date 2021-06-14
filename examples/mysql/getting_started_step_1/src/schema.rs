diesel::table! {
    posts (id) {
        id -> Integer,
        title -> Varchar,
        body -> Text,
        published -> Bool,
    }
}

diesel::table!{
    companys(company_id){
        company_id -> Integer,
        company_code -> Varchar,
        company_name -> Varchar,
    }
}