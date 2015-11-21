no_arg_sql_function!(now, Timestamp);
numeric_expr!(now);
sql_function!(date, (x: Timestamp) -> Date);
generic_numeric_expr!(date, A);
