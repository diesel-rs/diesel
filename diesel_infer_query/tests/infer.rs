// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel_infer_query::{Backend, IsNull, SchemaField, SchemaResolver};
use std::collections::HashMap;

#[derive(Hash, Eq, PartialEq, Debug)]
struct Key {
    schema: Option<String>,
    table: String,
    field: String,
}

struct Resolver {
    data: HashMap<Key, Field>,
}

impl<const N: usize> From<[(&'static str, &'static str, &'static str, IsNull); N]> for Resolver {
    fn from(value: [(&'static str, &'static str, &'static str, IsNull); N]) -> Self {
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

impl<const N: usize> From<[(&'static str, &'static str, IsNull); N]> for Resolver {
    fn from(value: [(&'static str, &'static str, IsNull); N]) -> Self {
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
    is_null: IsNull,
    name: &'static str,
}

impl SchemaResolver for Resolver {
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
        field_name: &str,
    ) -> Result<
        &'s dyn diesel_infer_query::SchemaField,
        Box<dyn std::error::Error + Send + Sync + 'static>,
    > {
        let Some(query_relation) = query_relation else {
            panic!(
                "Expected to get unnamed query relation from {:?}",
                self.data.keys()
            )
        };
        let key = Key {
            schema: relation_schema.map(|s| s.to_owned()),
            table: query_relation.to_string(),
            field: field_name.into(),
        };
        let s = self
            .data
            .get(&key)
            .unwrap_or_else(|| panic!("Expected to get {key:?} from {:?}", self.data.keys()));
        Ok(s)
    }

    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let Some(query_relation) = query_relation else {
            panic!(
                "Expected to get unnamed query relation from {:?}",
                self.data.keys()
            )
        };
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
        res.sort_by(|a, b| a.name().cmp(&b.name()));

        Ok(res)
    }
}

impl SchemaField for Field {
    fn is_nullable(&self) -> IsNull {
        self.is_null
    }

    fn name(&self) -> Option<&str> {
        Some(self.name)
    }
}

#[track_caller]
fn check_infer<const N: usize>(
    def: &'static str,
    expected: [IsNull; N],
    resolver: impl Into<Resolver>,
) {
    let mut resolver = resolver.into();
    let res = diesel_infer_query::parse_view_def(def, Backend::Sqlite);

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
        [IsNull::NotNullable, IsNull::NotNullable, IsNull::IsNullable],
        (),
    );
}

#[test]
pub(crate) fn view_with_literals_cast() {
    check_infer(
        "CREATE VIEW test AS SELECT 1::text AS foo, 'foo' AS bar, NULL::text",
        [IsNull::NotNullable, IsNull::NotNullable, IsNull::IsNullable],
        (),
    );
}

#[test]
pub(crate) fn simple_table() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, name, hair_color as hair_colour FROM users",
        [IsNull::NotNullable, IsNull::NotNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::NotNullable),
            ("users", "hair_color", IsNull::IsNullable),
        ],
    );

    check_infer(
        "CREATE VIEW test AS SELECT users.id, name, hair_color as hair_colour FROM users",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::IsNullable),
            ("users", "hair_color", IsNull::IsNullable),
        ],
    );
}

#[test]
pub(crate) fn simple_table_with_alias() {
    check_infer(
        "CREATE VIEW test AS SELECT u.id, name, u.hair_color AS hair_colour FROM users as u",
        [IsNull::NotNullable, IsNull::NotNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::NotNullable),
            ("users", "hair_color", IsNull::IsNullable),
        ],
    );
}

#[test]
pub(crate) fn simple_left_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users LEFT JOIN posts ON posts.user_id = users.id",
        [IsNull::NotNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("posts", "id", IsNull::NotNullable),
        ],
    );

    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users LEFT OUTER JOIN posts ON posts.user_id = users.id",
        [IsNull::NotNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("posts", "id", IsNull::NotNullable),
        ],
    );
}

#[test]
pub(crate) fn simple_inner_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users INNER JOIN posts ON posts.user_id = users.id",
        [IsNull::NotNullable, IsNull::NotNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("posts", "id", IsNull::NotNullable),
        ],
    );

    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id FROM users JOIN posts ON posts.user_id = users.id",
        [IsNull::NotNullable, IsNull::NotNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("posts", "id", IsNull::NotNullable),
        ],
    );
}

#[test]
pub(crate) fn nested_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, posts.id, comments.id FROM users LEFT JOIN posts ON posts.user_id = users.id INNER JOIN comments ON comments.post_id = posts.id",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("posts", "id", IsNull::NotNullable),
            ("comments", "id", IsNull::NotNullable),
        ],
    );
}

#[test]
pub(crate) fn operations() {
    check_infer(
        "CREATE VIEW test AS SELECT 1 + 1, 1+NULL, NULL+1, NULL + NULL",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        (),
    );
}

#[test]
fn is_null_and_is_not_null() {
    check_infer(
        "CREATE VIEW test AS SELECT NULL IS NOT NULL, NULL IS NULL",
        [IsNull::NotNullable, IsNull::NotNullable],
        (),
    );
}

#[test]
fn functions() {
    check_infer(
        "CREATE VIEW test AS SELECT count(*), COUNT(id), sum(id) FROM users",
        [IsNull::NotNullable, IsNull::NotNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn wildcard_select() {
    check_infer(
        "CREATE VIEW test AS SELECT * FROM users",
        [IsNull::NotNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::IsNullable),
        ],
    );
}

#[test]
fn qualified_wildcard_select() {
    check_infer(
        "CREATE VIEW test AS SELECT users.* FROM users",
        [IsNull::NotNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::IsNullable),
        ],
    );
}

#[test]
fn qualified_wildcard_select_left_join() {
    check_infer(
        "CREATE VIEW test AS SELECT users.*, posts.* FROM users LEFT JOIN posts ON users.id = posts.user_id",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::IsNullable),
            ("posts", "id", IsNull::NotNullable),
            ("posts", "name", IsNull::IsNullable),
        ],
    );
}

#[test]
fn is_distinct_from() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' IS DISTINCT FROM NULL, 'def' IS NOT DISTINCT FROM NULL",
        [IsNull::NotNullable, IsNull::NotNullable],
        (),
    )
}

#[test]
fn like() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' LIKE 'foo', 'cde' LIKE NULL, \
              'fgh' ILIKE '%', 'ijk' ILIKE NULL, 'abc' NOT LIKE '%', NULL NOT LIKE '%'",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
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
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        (),
    )
}

#[test]
fn similar_to() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' SIMILAR TO 'cde', 'ABC' NOT SIMILAR TO NULL, NULL SIMILAR TO 'abc'",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::IsNullable],
        (),
    )
}

#[test]
fn regexp() {
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' REGEXP 'abc', NULL REGEXP 'abc', 'abc' REGEXP NULL,\
         'abc' RLIKE 'abc', NULL RLIKE 'abc'",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
        ],
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
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::IsNullable],
        (),
    );
}

#[test]
fn in_list() {
    check_infer(
        "CREATE VIEW test AS SELECT
                  name IN (1, 2, 4), \
                  NULL IN (1, 2, 3), \
                  name IN (1, 3, NULL), \
                  hair_color IN (1, 2, 3) \
              FROM users",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("users", "name", IsNull::NotNullable),
            ("users", "hair_color", IsNull::IsNullable),
        ],
    );
}

#[test]
fn not_in_list() {
    check_infer(
        "CREATE VIEW test AS SELECT
                  name NOT IN (1, 2, 4), \
                  NULL NOT IN (1, 2, 3), \
                  name NOT IN (1, 3, NULL), \
                  hair_color NOT IN (1, 2, 3) \
                  FROM users",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("users", "name", IsNull::NotNullable),
            ("users", "hair_color", IsNull::IsNullable),
        ],
    );
}

#[test]
fn in_subquery() {
    check_infer(
        "CREATE VIEW test AS SELECT \
                          name IN (SELECT posts.name FROM posts), \
                          name IN (SELECT posts.body FROM posts), \
                          hair_color IN (SELECT posts.name FROM posts), \
                          name IN (SELECT users.hair_color FROM posts) \
                      FROM users",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("users", "name", IsNull::NotNullable),
            ("users", "hair_color", IsNull::IsNullable),
            ("posts", "name", IsNull::NotNullable),
            ("posts", "body", IsNull::IsNullable),
        ],
    );
}

#[test]
fn not_in_subquery() {
    check_infer(
        "CREATE VIEW test AS SELECT \
                          name NOT IN (SELECT posts.name FROM posts), \
                          name NOT IN (SELECT posts.body FROM posts), \
                          hair_color NOT IN (SELECT posts.name FROM posts), \
                          name NOT IN (SELECT users.hair_color FROM posts) \
                      FROM users",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("users", "name", IsNull::NotNullable),
            ("users", "hair_color", IsNull::IsNullable),
            ("posts", "name", IsNull::NotNullable),
            ("posts", "body", IsNull::IsNullable),
        ],
    );
}

#[test]
fn nested() {
    check_infer(
        "CREATE VIEW test AS SELECT \
                          (1 + 2) - 3, \
                          (NULL + 2) - 3, \
                          (1 + 3) - NULL, \
                          3 - (1 + 3), \
                          NULL - (1 + 3), \
                          3 - (NULL + 3)",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        (),
    );
}

#[test]
fn subquery() {
    check_infer(
        "CREATE VIEW test AS SELECT \
                  (SELECT users.name FROM users WHERE users.id = posts.user_id), \
                  (SELECT users.hair_color FROM users WHERE users.id = posts.user_id), \
                  (SELECT posts.body FROM users WHERE users.id = posts.user_id), \
                  (SELECT posts.title FROM users WHERE users.id = posts.user_id)
              FROM posts",
        [
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("users", "name", IsNull::NotNullable),
            ("users", "hair_color", IsNull::IsNullable),
            ("posts", "body", IsNull::NotNullable),
            ("posts", "title", IsNull::IsNullable),
        ],
    );
}

#[test]
fn with_cte() {
    check_infer(
        "CREATE VIEW test AS WITH source AS (\
             SELECT a, b FROM table1 WHERE c > 1\
         ) SELECT t1.col1, s.a, s.b, count(*) FROM users t1 \
         INNER JOIN source s ON t1.id = s.a GROUP BY t1.col1, s.b",
        [
            IsNull::NotNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
        ],
        [
            ("users", "col1", IsNull::NotNullable),
            ("table1", "a", IsNull::NotNullable),
            ("table1", "b", IsNull::IsNullable),
        ],
    );
}

#[test]
fn multi_level_cte() {
    check_infer(
        "CREATE VIEW test AS WITH source AS (\
             SELECT a, b FROM table1 WHERE c > 1\
             ), \
             intermediate AS (SELECT a + b AS one, a, b, count(*) as c FROM source) \
         SELECT t1.col1, s.a, s.b, count(*), s.one, s.c FROM users t1 \
         INNER JOIN intermediate s ON t1.id = s.a GROUP BY t1.col1, s.b",
        [
            IsNull::NotNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
        ],
        [
            ("users", "col1", IsNull::NotNullable),
            ("table1", "a", IsNull::NotNullable),
            ("table1", "b", IsNull::IsNullable),
        ],
    );
}

#[test]
fn casting() {
    check_infer(
        "CREATE VIEW test AS SELECT '123'::integer AS id, CAST(price AS text), name::varchar, CAST(name AS text) FROM items",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::NotNullable,
        ],
        [
            ("items", "price", IsNull::IsNullable),
            ("items", "name", IsNull::NotNullable),
        ],
    )
}

#[test]
fn subquery_source() {
    check_infer(
        "CREATE VIEW test AS SELECT t1.some_col, t1.other_col, users.name FROM (\
         SELECT some_col, other_col FROM posts WHERE posts.id < 10) AS t1 \
         INNER JOIN users ON users.id = t1.some_col",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::NotNullable],
        [
            ("posts", "some_col", IsNull::NotNullable),
            ("posts", "other_col", IsNull::IsNullable),
            ("users", "name", IsNull::NotNullable),
        ],
    );
}

#[test]
fn union() {
    check_infer(
        "CREATE VIEW test AS SELECT id, name, hair_color, age FROM old_users WHERE is_active = TRUE \
         UNION \
         SELECT user_id, display_name, hair, age FROM new_members",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("old_users", "id", IsNull::NotNullable),
            ("old_users", "name", IsNull::NotNullable),
            ("old_users", "hair_color", IsNull::IsNullable),
            ("old_users", "age", IsNull::IsNullable),
            ("new_members", "user_id", IsNull::NotNullable),
            ("new_members", "display_name", IsNull::IsNullable),
            ("new_members", "hair", IsNull::NotNullable),
            ("new_members", "age", IsNull::IsNullable),
        ],
    );
}

#[test]
fn intersect() {
    check_infer(
        "CREATE VIEW test AS SELECT id, name, hair_color, age FROM old_users WHERE is_active = TRUE \
         INTERSECT \
         SELECT user_id, display_name, hair, age FROM new_members",
        [
            IsNull::NotNullable,
            IsNull::NotNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
        ],
        [
            ("old_users", "id", IsNull::NotNullable),
            ("old_users", "name", IsNull::NotNullable),
            ("old_users", "hair_color", IsNull::IsNullable),
            ("old_users", "age", IsNull::IsNullable),
            ("new_members", "user_id", IsNull::NotNullable),
            ("new_members", "display_name", IsNull::IsNullable),
            ("new_members", "hair", IsNull::NotNullable),
            ("new_members", "age", IsNull::IsNullable),
        ],
    );
}

#[test]
fn except() {
    check_infer(
        "CREATE VIEW test AS SELECT id, name, hair_color, age FROM old_users WHERE is_active = TRUE \
         EXCEPT \
         SELECT user_id, display_name, hair, age FROM new_members",
        [
            IsNull::NotNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("old_users", "id", IsNull::NotNullable),
            ("old_users", "name", IsNull::NotNullable),
            ("old_users", "hair_color", IsNull::IsNullable),
            ("old_users", "age", IsNull::IsNullable),
            ("new_members", "user_id", IsNull::NotNullable),
            ("new_members", "display_name", IsNull::IsNullable),
            ("new_members", "hair", IsNull::NotNullable),
            ("new_members", "age", IsNull::IsNullable),
        ],
    );
}

#[test]
fn recursive_cte() {
    check_infer(
        "CREATE VIEW test AS WITH RECURSIVE cte (id, name, parent_id) AS (\
               SELECT employee_id, employee_name, parent_id FROM employees WHERE parent_id IS NULL \
               UNION ALL \
               SELECT e.employee_id, e.employee_name, e.parent_id FROM employees e JOIN cte ON e.parent_id = cte.id \
               ) SELECT id, name, parent_id FROM cte",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::NotNullable],
        [
            ("employees", "employee_id", IsNull::NotNullable),
            ("employees", "employee_name", IsNull::IsNullable),
            ("employees", "parent_id", IsNull::NotNullable),
        ],
    );
}

#[test]
fn complex_cte_subquery_set_ops() {
    check_infer(
           "CREATE VIEW test AS WITH \
               stage1 AS (SELECT a, b, c, d FROM sources WHERE c > 1), \
               stage2 AS (SELECT a, b, c, d FROM other_sources WHERE c > 1), \
               union_set AS (SELECT a, b, c, d FROM stage1 UNION SELECT a, b, c, d FROM stage2), \
               cte_nested AS (
                   SELECT t.a, t.b, t.c, t.d, s.col1, s.col2 FROM union_set t JOIN (SELECT col1, col2 FROM small_table LIMIT 1) s ON t.a = s.col
               ) \
           SELECT a, b, c, d, col1, col2 FROM cte_nested",
                [IsNull::NotNullable, IsNull::IsNullable, IsNull::IsNullable, IsNull::IsNullable, IsNull::NotNullable, IsNull::IsNullable],
        [
            ("sources", "a", IsNull::NotNullable),
            ("sources", "b", IsNull::IsNullable),
            ("sources", "c", IsNull::NotNullable),
            ("sources", "d", IsNull::IsNullable),
            ("other_sources", "a", IsNull::NotNullable),
            ("other_sources", "b", IsNull::NotNullable),
            ("other_sources", "c", IsNull::IsNullable),
            ("other_sources", "d", IsNull::IsNullable),
            ("small_table", "col1", IsNull::NotNullable),
            ("small_table", "col2", IsNull::IsNullable),
        ]
       );
}

#[test]
fn coalesce_function() {
    // COALESCE should be NOT NULL if all arguments are NOT NULL
    // (returns the first non-null value)
    check_infer(
        "CREATE VIEW test AS SELECT coalesce(foo.is_null, NULL, 3) AS result FROM foo",
        [IsNull::NotNullable],
        [("foo", "is_null", IsNull::IsNullable)],
    );
}

#[test]
fn unnamed_query_source() {
    check_infer(
        "SELECT * FROM (SELECT 1 as dummy)",
        [IsNull::NotNullable],
        (),
    );
}

#[test]
fn cte_with_wildcard() {
    check_infer(
        "CREATE VIEW test AS WITH c AS (SELECT * FROM users) SELECT id, name FROM c",
        [IsNull::NotNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::IsNullable),
        ],
    );
}

#[test]
fn from_subquery_with_wildcard() {
    check_infer(
        "CREATE VIEW test AS SELECT u.id FROM (SELECT * FROM users) u",
        [IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn nested_subqueries_with_same_name() {
    check_infer(
        "CREATE VIEW test AS WITH \
                  query_1 AS (SELECT dummy.id FROM (SELECT 1 AS id) as dummy), \
                  query_2 AS (SELECT dummy.id FROM (SELECT NULL AS id) as dummy) \
              SELECT id FROM query_1 UNION SELECT id FROM query_2",
        [IsNull::IsNullable],
        (),
    );
}

#[test]
fn query_source_names_ignore_case() {
    // SQLite matches names case-insensitively and stores view definitions as written
    check_infer(
        "CREATE VIEW test AS SELECT USERS.*, Posts.id, comments.id FROM users \
         LEFT JOIN posts ON POSTS.user_id = Users.id \
         INNER JOIN comments ON comments.post_id = POSTS.id",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
        ],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::IsNullable),
            ("posts", "id", IsNull::NotNullable),
            ("comments", "id", IsNull::NotNullable),
        ],
    );
    // likewise for common table expressions and derived tables, and their columns
    check_infer(
        "CREATE VIEW test AS WITH C AS (SELECT id AS Id FROM users) \
         SELECT c.ID, D.x FROM c, (SELECT 1 AS X) AS d",
        [IsNull::NotNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn case_without_else() {
    // without an ELSE, the result is NULL when no branch matches
    check_infer(
        "CREATE VIEW test AS SELECT CASE WHEN 1 = 1 THEN 1 END, CASE 1 WHEN 1 THEN 1 END",
        [IsNull::IsNullable, IsNull::IsNullable],
        (),
    );
}
