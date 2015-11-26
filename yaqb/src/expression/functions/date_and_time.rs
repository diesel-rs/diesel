use types::*;

no_arg_sql_function!(now, Timestamp);
operator_allowed!(now, Add, add);
operator_allowed!(now, Sub, sub);
sql_function!(date, date_t, (x: Timestamp) -> Date);
