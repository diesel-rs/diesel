use types::*;

no_arg_sql_function!(now, Timestamp, "Represents the SQL NOW() function");
operator_allowed!(now, Add, add);
operator_allowed!(now, Sub, sub);
sql_function!(date, date_t, (x: Timestamp) -> Date,
"Represents the SQL DATE() function. The argument should be a Timestamp
expression, and the return value will be an expression of type Date");
