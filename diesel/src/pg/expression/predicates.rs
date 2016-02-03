use pg::Pg;

infix_predicate!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", backend: Pg);
