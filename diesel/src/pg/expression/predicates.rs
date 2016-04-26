use pg::Pg;

infix_predicate!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", backend: Pg);
infix_predicate!(OverlapsWith, " && ", backend: Pg);
infix_predicate!(Contains, " @> ", backend: Pg);
infix_predicate!(IsContainedBy, " <@ ", backend: Pg);
