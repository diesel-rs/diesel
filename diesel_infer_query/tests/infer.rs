// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use diesel_infer_query::{SchemaField, SchemaResolver};

#[derive(Hash, Eq, PartialEq)]
struct Key {
    schema: Option<String>,
    table: String,
    field: String,
}

struct Resolver {
    data: HashMap<Key, Field>,
}

impl<const N: usize> From<[(&'static str, &'static str, &'static str, bool); N]> for Resolver {
    fn from(value: [(&'static str, &'static str, &'static str, bool); N]) -> Self {
        let data = value
            .into_iter()
            .map(|(schema, rel, field, null)| {
                (
                    Key {
                        schema: Some(schema.into()),
                        table: rel.into(),
                        field: field.into(),
                    },
                    Field {
                        is_null: null,
                        name: field,
                    },
                )
            })
            .collect();
        Self { data }
    }
}

impl<const N: usize> From<[(&'static str, &'static str, bool); N]> for Resolver {
    fn from(value: [(&'static str, &'static str, bool); N]) -> Self {
        let data = value
            .into_iter()
            .map(|(rel, field, null)| {
                (
                    Key {
                        schema: None,
                        table: rel.into(),
                        field: field.into(),
                    },
                    Field {
                        is_null: null,
                        name: field,
                    },
                )
            })
            .collect();
        Self { data }
    }
}

impl From<()> for Resolver {
    fn from(_value: ()) -> Self {
        Self {
            data: Default::default(),
        }
    }
}

struct Field {
    is_null: bool,
    name: &'static str,
}

impl SchemaResolver for Resolver {
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: &str,
        field_name: &str,
    ) -> Result<
        &'s dyn diesel_infer_query::SchemaField,
        Box<dyn std::error::Error + Send + Sync + 'static>,
    > {
        let key = Key {
            schema: relation_schema.map(|s| s.to_owned()),
            table: query_relation.to_string(),
            field: field_name.into(),
        };
        let s = self.data.get(&key).unwrap();
        Ok(s)
    }

    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: &str,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut res = self
            .data
            .iter()
            .filter_map(|(k, v)| {
                (k.schema.as_deref() == relation_schema && k.table == query_relation)
                    .then_some(v as &dyn SchemaField)
            })
            .collect::<Vec<_>>();
        // need to sort here otherwise we get a random column
        // order by the hashmap
        //
        // This now assumes that columns are alphabetically sorted for tables
        res.sort_by(|a, b| a.name().cmp(b.name()));

        Ok(res)
    }
}

impl SchemaField for Field {
    fn is_nullable(&self) -> bool {
        self.is_null
    }

    fn name(&self) -> &str {
        self.name
    }
}

#[track_caller]
fn check_infer<const N: usize>(
    def: &'static str,
    expected: [Option<bool>; N],
    resolver: impl Into<Resolver>,
) {
    let mut resolver = resolver.into();
    let res = diesel_infer_query::parse_view_def(def);

    assert!(
        res.is_ok(),
        "Failed to infer SQL with error: {}",
        res.unwrap_err()
    );
    let mut view_def = res.unwrap();
    view_def.resolve_references(&mut resolver).unwrap();
    assert_eq!(view_def.field_count(), N);

    let res = view_def.infer_nullability(&mut resolver);
    assert!(
        res.is_ok(),
        "Failed to infer nullablity: {}",
        res.unwrap_err()
    );
    let res = res.unwrap();
    assert_eq!(res, expected);
}

#[test]
pub(crate) fn view_with_literals() {
    check_infer(
        "CREATE VIEW test AS SELECT 1 AS foo, 'foo' AS bar, NULL",
        [Some(false), Some(false), Some(true)],
        (),
    );
}

#[test]
pub(crate) fn view_with_literals_cast() {
    check_infer(
        "CREATE VIEW test AS SELECT 1::text AS foo, 'foo' AS bar, NULL::text",
        [Some(false), Some(false), Some(true)],
        (),
    );
}

#[test]
pub(crate) fn simple_table() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, name, hair_color as hair_colour FROM users",
        [Some(false), Some(false), Some(true)],
        [
            ("users", "id", false),
            ("users", "name", false),
            ("users", "hair_color", true),
        ],
    );

    check_infer(
        "CREATE VIEW test AS SELECT users.id, name, hair_color as hair_colour FROM users",
        [Some(false), Some(true), Some(true)],
        [
            ("users", "id", false),
            ("users", "name", true),
            ("users", "hair_color", true),
        ],
    );
}

#[test]
pub(crate) fn simple_table_with_alias() {
    check_infer(
        "CREATE VIEW test AS SELECT u.id, name, u.hair_color AS hair_colour FROM users as u",
        [Some(false), Some(false), Some(true)],
        [
            ("users", "id", false),
            ("users", "name", false),
            ("users", "hair_color", true),
        ],
    );
}

#[test]
pub(crate) fn simple_left_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users LEFT JOIN posts ON posts.user_id = users.id",
        [Some(false), Some(true)],
        [("users", "id", false), ("posts", "id", false)],
    );

    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users LEFT OUTER JOIN posts ON posts.user_id = users.id",
        [Some(false), Some(true)],
        [("users", "id", false), ("posts", "id", false)],
    );
}

#[test]
pub(crate) fn simple_inner_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users INNER JOIN posts ON posts.user_id = users.id",
        [Some(false), Some(false)],
        [("users", "id", false), ("posts", "id", false)],
    );

    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users JOIN posts ON posts.user_id = users.id",
        [Some(false), Some(false)],
        [("users", "id", false), ("posts", "id", false)],
    );
}

#[test]
pub(crate) fn nested_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id, comments.id FROM users LEFT JOIN posts ON posts.user_id = users.id INNER JOIN comments ON comments.post_id = posts.id",
        [Some(false), Some(true), Some(true)],
        [
            ("users", "id", false),
            ("posts", "id", false),
            ("comments", "id", false),
        ],
    );
}

#[test]
pub(crate) fn operations() {
    check_infer(
        "CREATE VIEW test AS SELECT 1+ 1, 1+NULL, NULL+1, NULL + NULL",
        [Some(false), Some(true), Some(true), Some(true)],
        (),
    );
}

#[test]
fn is_null_and_is_not_null() {
    check_infer(
        "CREATE VIEW test AS SELECT NULL IS NOT NULL, NULL IS NULL",
        [Some(false), Some(false)],
        (),
    );
}

#[test]
fn functions() {
    check_infer(
        "CREATE VIEW test AS SELECT count(*), COUNT(id), sum(id) FROM users",
        [Some(false), Some(false), Some(true)],
        [("users", "id", false)],
    );
}

#[test]
fn wildcard_select() {
    check_infer(
        "CREATE VIEW test AS SELECT * FROM users",
        [Some(false), Some(true)],
        [("users", "id", false), ("users", "name", true)],
    );
}

#[test]
fn qualified_wildcard_select() {
    check_infer(
        "CREATE VIEW test AS SELECT users.* FROM users",
        [Some(false), Some(true)],
        [("users", "id", false), ("users", "name", true)],
    );
}

#[test]
fn qualified_wildcard_select_left_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.*, posts.* FROM users LEFT JOIN posts ON users.id = posts.user_id",
        [Some(false), Some(true), Some(true), Some(true)],
        [
            ("users", "id", false),
            ("users", "name", true),
            ("posts", "id", false),
            ("posts", "name", true),
        ],
    );
}

#[test]
fn is_distinct_from() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' IS DISTINCT FROM NULL, 'def' IS NOT DISTINCT FROM NULL",
        [Some(false), Some(false)],
        (),
    )
}

#[test]
fn like() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' LIKE 'foo', 'cde' LIKE NULL, \
              'fgh' ILIKE '%', 'ijk' ILIKE NULL, 'abc' NOT LIKE '%', NULL NOT LIKE '%'",
        [
            Some(false),
            Some(true),
            Some(false),
            Some(true),
            Some(false),
            Some(true),
        ],
        (),
    );
}

#[test]
fn between() {
    check_infer(
        "CREATE VIEW test AS SELECT 1 BETWEEN 0 AND 10, 1 BETWEEN NULL AND 25, \
             1 NOT BETWEEN 0 AND 10, 1 NOT BETWEEN NULL AND 25, NULL BETWEEN 1 AND 2, \
             1 BETWEEN 2 AND NULL",
        [
            Some(false),
            Some(true),
            Some(false),
            Some(true),
            Some(true),
            Some(true),
        ],
        (),
    )
}

#[test]
fn similar_to() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' SIMILAR TO 'cde', 'ABC' NOT SIMILAR TO NULL, NULL SIMILAR TO 'abc'",
        [Some(false), Some(true), Some(true)],
        (),
    )
}

#[test]
fn regexp() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' REGEXP 'abc', NULL REGEXP 'abc', 'abc' REGEXP NULL,\
        'abc' RLIKE 'abc', NULL RLIKE 'abc'",
        [Some(false), Some(true), Some(true), Some(false), Some(true)],
        (),
    )
}

#[test]
fn case_when() {
    check_infer(
        "CREATE VIEW test AS SELECT \
              CASE WHEN 1 = 1 THEN 1 WHEN NULL THEN 1 ELSE 1 END,
              CASE WHEN 1 = 1 THEN 1 WHEN NULL THEN 1 ELSE NULL END,
              CASE WHEN 1 = 1 THEN NULL WHEN NULL THEN 1 ELSE 1 END",
        [Some(false), Some(true), Some(true)],
        (),
    );
}
