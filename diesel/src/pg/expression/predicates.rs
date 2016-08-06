use pg::Pg;
use query_builder::QueryBuilder;

infix_predicate!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", backend: Pg);
infix_predicate!(OverlapsWith, " && ", backend: Pg);
infix_predicate!(Contains, " @> ", backend: Pg);
infix_predicate!(IsContainedBy, " <@ ", backend: Pg);
postfix_expression!(NullsFirst, " NULLS FIRST", ());
postfix_expression!(NullsLast, " NULLS LAST", ());
