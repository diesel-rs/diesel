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
                    Field { is_null: null },
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
                    Field { is_null: null },
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
}

impl SchemaField for Field {
    fn is_nullable(&self) -> bool {
        self.is_null
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
    let view_def = res.unwrap();
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
