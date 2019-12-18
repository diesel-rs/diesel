simple_clause!(NoGroupByClause, GroupByClause, " GROUP BY ");

pub trait ValidGroupByClause {
    type Expressions;
}

impl ValidGroupByClause for NoGroupByClause {
    type Expressions = ();
}

impl<GB> ValidGroupByClause for GroupByClause<GB> {
    type Expressions = GB;
}
