use pg::Pg;

infix_operator!(IsDistinctFrom, " IS DISTINCT FROM ", backend: Pg);
infix_operator!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", backend: Pg);
infix_operator!(OverlapsWith, " && ", backend: Pg);
infix_operator!(Contains, " @> ", backend: Pg);
infix_operator!(IsContainedBy, " <@ ", backend: Pg);
infix_operator!(ILike, " ILIKE ", backend: Pg);
infix_operator!(NotILike, " NOT ILIKE ", backend: Pg);
postfix_operator!(NullsFirst, " NULLS FIRST", (), backend: Pg);
postfix_operator!(NullsLast, " NULLS LAST", (), backend: Pg);
