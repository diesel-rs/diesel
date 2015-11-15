no_arg_sql_function!(now, Timestamp);
sql_function!(date, (x: Timestamp) -> Date);
