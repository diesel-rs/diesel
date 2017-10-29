use sqlite::Sqlite;

diesel_postfix_operator!(CollateBinary, " COLLATE BINARY", backend: Sqlite);
diesel_postfix_operator!(CollateNoCase, " COLLATE NOCASE", backend: Sqlite);
diesel_postfix_operator!(CollateRTrim, " COLLATE RTRIM", backend: Sqlite);
