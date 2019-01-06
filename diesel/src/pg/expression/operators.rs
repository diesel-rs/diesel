use crate::pg::Pg;

diesel_infix_operator!(IsDistinctFrom, " IS DISTINCT FROM ", backend: Pg);
diesel_infix_operator!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", backend: Pg);
diesel_infix_operator!(OverlapsWith, " && ", backend: Pg);
diesel_infix_operator!(Contains, " @> ", backend: Pg);
diesel_infix_operator!(IsContainedBy, " <@ ", backend: Pg);
diesel_infix_operator!(ILike, " ILIKE ", backend: Pg);
diesel_infix_operator!(NotILike, " NOT ILIKE ", backend: Pg);
diesel_postfix_operator!(NullsFirst, " NULLS FIRST", (), backend: Pg);
diesel_postfix_operator!(NullsLast, " NULLS LAST", (), backend: Pg);
