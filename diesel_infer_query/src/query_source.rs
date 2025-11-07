// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! This module contains datastructures and methods
//! to handle parsing and analyzing the QuerySource (`FROM` clause)
//! part of a query.
//!
//! Here we are mostly interested in understanding the relation of
//! the different tables/views/… between each other. So how they are joined
//! and in which order

use crate::error::{Error, Result};
use sqlparser::ast::ObjectNamePart;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::ControlFlow;

/// The kind of join
///
/// This information is relevant to
/// understand whether or not a specific
/// expression might be null
#[derive(Debug, Clone, Copy)]
pub(crate) enum JoinKind {
    /// A `INNER JOIN`
    Inner,
    // A `LEFT [OUTER] JOIN`
    Left,
}

/// Information about a specific join
#[derive(Debug)]
pub(crate) struct Join {
    /// To which other query source the current source is joined
    ///
    /// This field contains the name of the query source as used in other
    /// parts of the query, so if applicable the aliased name
    ///
    /// You can use that directly to lookup the query source via the query
    /// source lookup map
    pub(crate) to: String,
    /// The kind of join
    pub(crate) kind: JoinKind,
}

/// A query source
///
/// This can be a table or view
///
// Possibly that needs to be an enum later
// to handle subqueries, VALUES clauses, etc
#[derive(Debug)]
pub(crate) struct QuerySource<'a> {
    /// The schema of the query source
    pub(crate) schema: Option<&'a str>,
    /// The name of the query source
    pub(crate) name: &'a str,
    /// The alias that is used to refer to this query source in the query
    #[expect(dead_code, reason = "its there for later")]
    pub(crate) alias: Option<&'a str>,
    /// A possible join that was used to combine this query source with others
    pub(crate) join: Option<Join>,
}

impl<'a> QuerySource<'a> {
    /// Construct a query source from the given parser statement
    ///
    /// The `out` map contains a lookup list indexed by the name of the query source
    /// as used in the query
    pub(crate) fn fill_from_table_with_joins(
        out: &mut HashMap<&'a str, QuerySource<'a>>,
        table_with_joins: &'a sqlparser::ast::TableWithJoins,
    ) -> Result<()> {
        // first resolve the table itself
        Self::fill_from_table_factor(&table_with_joins.relation, out)?;
        // then any joins to this table
        for join in &table_with_joins.joins {
            Self::fill_from_join(out, join)?;
        }
        Ok(())
    }

    /// Check if this specific query source or any query source in the join chain
    /// used to include this query source is joined via a `LEFT JOIN`
    pub(crate) fn contains_left_join(
        &self,
        query_source_lookup: &&HashMap<&str, QuerySource<'_>>,
    ) -> Result<bool> {
        if let Some(join) = &self.join {
            match join.kind {
                JoinKind::Inner => {
                    let source = query_source_lookup.get(join.to.as_str()).ok_or_else(|| {
                        Error::InvalidQuerySource {
                            query_source: join.to.clone(),
                        }
                    })?;
                    source.contains_left_join(query_source_lookup)
                }
                JoinKind::Left => Ok(true),
            }
        } else {
            Ok(false)
        }
    }

    fn fill_from_table_factor(
        s: &'a sqlparser::ast::TableFactor,
        out: &mut HashMap<&'a str, QuerySource<'a>>,
    ) -> Result<()> {
        match s {
            sqlparser::ast::TableFactor::Table {
                name,
                alias,
                args: None,
                with_hints,
                version: None,
                with_ordinality: false,
                partitions,
                json_path: None,
                sample: None,
                index_hints,
            }
            // we try to be conservative with what we support here
            // so disallow almost everything for now
            if with_hints.is_empty()
                && partitions.is_empty()
                && index_hints.is_empty()
                && name.0.len() <= 2
                && name
                    .0
                    .iter()
                    .all(|n| matches!(n, ObjectNamePart::Identifier(_))) =>
            {
                // for a plain query source we need to extract the name (including schema if exists)
                // and the alias for later usage
                let (name, schema) = match name.0.as_slice() {
                    // for a single part identifier we only have the table name
                    [ObjectNamePart::Identifier(name)] => (&name.value, None),
                    // for a two part identifier we also have the schema
                    [
                        ObjectNamePart::Identifier(schema),
                        ObjectNamePart::Identifier(name),
                    ] => (&name.value, Some(schema.value.as_str())),
                    // for now anythinge else is not supported, but that should be fine
                    _ => {
                        return Err(Error::UnsupportedSql {
                            msg: format!("Unsupported name: `{name}`"),
                        });
                    }
                };

                let alias = alias.as_ref().map(|a| a.name.value.as_str());
                 // if we have an alias the query source will be always referred in our query by the name of the alias
                // So use the name of the alias in that case, otherwise the name of the table.
                let lookup = alias.unwrap_or(name);
                out.insert(
                    lookup,
                    QuerySource {
                        schema,
                        name,
                        alias,
                        join: None,
                    },
                );
                Ok(())
            }
            sqlparser::ast::TableFactor::NestedJoin {
                table_with_joins,
                alias: None, // don't support aliases for joins yet
            } => {
                Self::fill_from_table_with_joins(out, table_with_joins)?;
                Ok(())
            }

            s => Err(Error::UnsupportedSql {
                msg: format!("Unsupported query source: `{s}`"),
            }),
        }
    }

    fn fill_from_join(
        out: &mut HashMap<&'a str, QuerySource<'a>>,
        join: &'a sqlparser::ast::Join,
    ) -> Result<()> {
        use sqlparser::ast::Visit;

        // get the inner parts
        let mut inner = HashMap::new();
        Self::fill_from_table_factor(&join.relation, &mut inner)?;

        // verify that the "inner parts" is in fact one part
        //
        // If we ever get more than one part here we need to come up with
        // a better solution to match the joins to the query sources
        if inner.len() != 1 {
            return Err(crate::error::Error::UnsupportedSql {
                msg: format!("Unsupported join clause: `{join}`"),
            });
        }

        // collect query source names used in the join expressions
        //
        // That's likely not 100% correct but a good first approximation
        let mut collector = IdentifierCollector::default();
        let kind = match &join.join_operator {
            // these are equivalent
            sqlparser::ast::JoinOperator::LeftOuter(l) | sqlparser::ast::JoinOperator::Left(l) => {
                let _ = l.visit(&mut collector);
                JoinKind::Left
            }
            // postgres rewrites `INNER JOIN` to just `JOIN`
            sqlparser::ast::JoinOperator::Inner(i) | sqlparser::ast::JoinOperator::Join(i) => {
                let _ = i.visit(&mut collector);
                JoinKind::Inner
            }
            // Anything else is not supported for now
            // maybe add support for more join kinds if they turn up later
            _d => {
                return Err(crate::error::Error::UnsupportedSql {
                    msg: format!("Unsupported join type: `{join}`"),
                });
            }
        };
        let mut join_expr = collector.idents;
        // remove the query source used in the join directly
        // So for `INNER JOIN posts ON posts.user_id = users.id`
        // remove `posts`
        for inner in inner.keys() {
            join_expr.remove(*inner);
        }
        // we should now have only one table left
        // that's the table we are joining to
        let to = if join_expr.len() == 1 {
            join_expr.into_iter().next().unwrap()
        } else {
            // otherwise this is a complicated join
            // we don't support yet
            return Err(crate::error::Error::UnsupportedSql {
                msg: format!("Unsupported join clause: `{join}`"),
            });
        };
        for (l, mut v) in inner {
            v.join = Some(Join {
                to: to.clone(),
                kind,
            });
            out.insert(l, v);
        }
        Ok(())
    }
}

#[derive(Default, Debug)]
pub(crate) struct IdentifierCollector {
    pub(crate) idents: HashSet<String>,
}

impl sqlparser::ast::Visitor for IdentifierCollector {
    type Break = ();

    fn pre_visit_expr(&mut self, expr: &sqlparser::ast::Expr) -> ControlFlow<Self::Break> {
        // we are just interested in identifiers
        if let sqlparser::ast::Expr::CompoundIdentifier(i) = expr
            && let [tbl, _] = i.as_slice()
        {
            self.idents.insert(tbl.value.clone());
        }
        ControlFlow::Continue(())
    }
}
