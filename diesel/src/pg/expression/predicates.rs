use pg::Pg;

infix_predicate!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", backend: Pg);
infix_predicate!(OverlapsWith, " && ", backend: Pg);
infix_predicate!(Contains, " @> ", backend: Pg);
infix_predicate!(IsContainedBy, " <@ ", backend: Pg);
infix_predicate!(ILike, " ILIKE ", backend: Pg);
infix_predicate!(NotILike, " NOT ILIKE ", backend: Pg);
postfix_expression!(NullsFirst, " NULLS FIRST", ());
postfix_expression!(NullsLast, " NULLS LAST", ());
