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
    check_infer_for(Backend::Sqlite, def, expected, resolver);
}

#[track_caller]
fn check_infer_for<const N: usize>(
    backend: Backend,
    def: &'static str,
    expected: [IsNull; N],
    resolver: impl Into<Resolver>,
) {
    let mut resolver = resolver.into();
    let res = diesel_infer_query::parse_view_def(def, backend);

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
    assert_eq!(res, expected, "{backend:?}");
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
        "CREATE VIEW test AS SELECT 1 = 1, 1 = NULL, NULL = 1, NULL = NULL",
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
    // SQLite evaluates LIKE with a function an application can replace
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' LIKE 'foo', 'abc' NOT LIKE '%', NULL LIKE 'foo'",
        [IsNull::IsNullable, IsNull::IsNullable, IsNull::IsNullable],
        (),
    );
    check_infer_for(
        Backend::Pg,
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
    check_infer_for(
        Backend::Pg,
        "CREATE VIEW test AS SELECT 'abc' SIMILAR TO 'cde', 'ABC' NOT SIMILAR TO NULL, NULL SIMILAR TO 'abc'",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::IsNullable],
        (),
    )
}

#[test]
fn regexp() {
    for backend in [Backend::Mysql, Backend::Mariadb] {
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT 'abc' REGEXP 'abc', NULL REGEXP 'abc', \
            'abc' RLIKE 'abc', NULL RLIKE 'abc'",
            [
                IsNull::NotNullable,
                IsNull::IsNullable,
                IsNull::NotNullable,
                IsNull::IsNullable,
            ],
            (),
        );
    }
    // SQLite's REGEXP and MATCH call functions only an application defines
    check_infer(
        "CREATE VIEW test AS SELECT 'abc' REGEXP 'abc', 'abc' MATCH 'abc'",
        [IsNull::IsNullable, IsNull::IsNullable],
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
                          ('a' || 'b') || 'c', \
                          (NULL || 'b') || 'c', \
                          ('a' || 'c') || NULL, \
                          'c' || ('a' || 'c'), \
                          NULL || ('a' || 'c'), \
                          'c' || (NULL || 'c')",
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
    // `intermediate` aggregates without GROUP BY, so it returns a row with NULL for `a`,
    // `b`, and `one` even when `source` is empty
    check_infer(
        "CREATE VIEW test AS WITH source AS (\
             SELECT a, b FROM table1 WHERE c > 1\
             ), \
             intermediate AS (SELECT a + b AS one, a, b, count(*) as c FROM source) \
         SELECT t1.col1, s.a, s.b, count(*), s.one, s.c FROM users t1 \
         INNER JOIN intermediate s ON t1.id = s.a GROUP BY t1.col1, s.b",
        [
            IsNull::NotNullable,
            IsNull::IsNullable,
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

#[test]
fn operators_returning_null_for_non_null_operands() {
    // `/` and `%` return NULL for a zero divisor in SQLite and MySQL, `->>` returns
    // NULL for a missing key, and SQLite turns a NaN result like `'1e999' - '1e999'`
    // into NULL
    check_infer(
        "CREATE VIEW test AS SELECT id / id, id % id, name ->> 'key', id + id, id - id, \
              id * id, id = id FROM users",
        [
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
        ],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::NotNullable),
        ],
    );
}

#[test]
fn is_distinct_from_followed_by_an_operator() {
    // sqlparser takes a comparison after `FROM` as the right operand, but SQLite reads
    // `a IS NOT DISTINCT FROM b = c` as `(a IS NOT DISTINCT FROM b) = c`, which can be
    // NULL. It binds AND looser, as sqlparser does, and `->>` tighter than `=`, which
    // sqlparser does not, so the `=` hides below the `->>` in the last one.
    for backend in [Backend::Sqlite, Backend::Pg] {
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT id IS DISTINCT FROM name AND hair_color, \
             id IS NOT DISTINCT FROM name = hair_color, id IS DISTINCT FROM name || hair_color, \
             id IS DISTINCT FROM name = hair_color ->> 'k' FROM users",
            [
                IsNull::IsNullable,
                IsNull::Unknown,
                IsNull::NotNullable,
                IsNull::Unknown,
            ],
            [
                ("users", "id", IsNull::NotNullable),
                ("users", "name", IsNull::NotNullable),
                ("users", "hair_color", IsNull::IsNullable),
            ],
        );
        // a unary minus binds tighter than IS in both, but SQLite binds IN as loosely as
        // IS and reads the last one as `(id IS DISTINCT FROM name) IN (NULL)`, NULL
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT id IS DISTINCT FROM -id, \
             id IS DISTINCT FROM name IN (NULL) FROM users",
            [IsNull::NotNullable, IsNull::Unknown],
            [
                ("users", "id", IsNull::NotNullable),
                ("users", "name", IsNull::NotNullable),
            ],
        );
    }
}

#[test]
fn bare_columns_in_aggregate_queries() {
    // Without GROUP BY an aggregate query returns one row even for an empty table,
    // and columns outside the aggregates are NULL in that row
    check_infer(
        "CREATE VIEW test AS SELECT id, id IS NULL, count(*) FROM users",
        [IsNull::IsNullable, IsNull::NotNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT *, max(id) FROM users",
        [IsNull::IsNullable, IsNull::IsNullable, IsNull::IsNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::NotNullable),
        ],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id FROM users HAVING count(*) >= 0",
        [IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // a bare column is NULL in that row wherever it appears, while a CASE over it can
    // still pick a branch without it
    check_infer(
        "CREATE VIEW test AS SELECT CAST(id AS TEXT), (id), id BETWEEN 1 AND 2, \
         CASE WHEN 1 THEN id ELSE id END, CASE id WHEN 1 THEN 2 ELSE 3 END, id IN (1, 2), \
         id IN (SELECT 1), 1 IN (SELECT users.id), count(*) FROM users",
        [
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::IsNullable,
            IsNull::NotNullable,
        ],
        [("users", "id", IsNull::NotNullable)],
    );
    // groups and window functions only see existing rows
    check_infer(
        "CREATE VIEW test AS SELECT id, count(*) FROM users GROUP BY id",
        [IsNull::NotNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id, count(*) OVER () FROM users",
        [IsNull::NotNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn aggregates_in_named_windows_make_bare_columns_nullable() {
    check_infer(
        "CREATE VIEW test AS SELECT id, count(*) OVER w FROM users WHERE 0 \
         WINDOW w AS (ORDER BY sum(id))",
        [IsNull::IsNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id, count(*) OVER w FROM users WHERE 0 \
         WINDOW w AS (PARTITION BY count(*))",
        [IsNull::IsNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id, count(*) OVER w2 FROM users WHERE 0 \
         WINDOW w1 AS (ORDER BY sum(id)), w2 AS (w1)",
        [IsNull::IsNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn grouping_sets_make_grouping_columns_nullable() {
    check_infer(
        "CREATE VIEW test AS SELECT id, count(*) FROM users GROUP BY ROLLUP(id)",
        [IsNull::IsNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id FROM users GROUP BY CUBE(id)",
        [IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id FROM users GROUP BY id GROUPING SETS ((id), ())",
        [IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // PostgreSQL's dialect reads them as grouping sets rather than calls
    for grouping in ["ROLLUP (id)", "CUBE (id)", "GROUPING SETS ((id), ())"] {
        let definition =
            format!("CREATE VIEW test AS SELECT id, count(*) FROM users GROUP BY {grouping}");
        check_infer_for(
            Backend::Pg,
            definition.leak(),
            [IsNull::IsNullable, IsNull::NotNullable],
            [("users", "id", IsNull::NotNullable)],
        );
    }
    // while PostgreSQL 19's GROUP BY ALL groups by the columns outside the aggregates
    check_infer_for(
        Backend::Pg,
        "CREATE VIEW test AS SELECT id, count(*) FROM users GROUP BY ALL",
        [IsNull::NotNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );

    // The SQLite parser dialect rejects syntaxes it does not represent.
    for definition in [
        "CREATE VIEW test AS SELECT id FROM users GROUP BY GROUPING SETS ((id), ())",
        "CREATE VIEW test AS SELECT id FROM users GROUP BY id WITH ROLLUP",
    ] {
        assert!(diesel_infer_query::parse_view_def(definition, Backend::Sqlite).is_err());
    }
}

#[test]
fn sqlite_percentile_aggregates_make_bare_columns_nullable() {
    check_infer(
        "CREATE VIEW test AS SELECT id, median(id) FROM users",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id, percentile(id, 50) FROM users",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id, percentile_cont(id, 0.5) FROM users",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT id, percentile_disc(id, 0.5) FROM users",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn multi_argument_min_and_max_are_scalar() {
    check_infer(
        "CREATE VIEW test AS SELECT id, min(id, 1), max(id, 1) FROM users",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // Scalar min still contains an aggregate, so its arguments must be traversed.
    check_infer(
        "CREATE VIEW test AS SELECT id, min(sum(id), 1) FROM users",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn nested_aggregate_ownership() {
    check_infer(
        "CREATE VIEW test AS SELECT users.id, \
         (SELECT count(*) FROM posts WHERE posts.user_id = users.id) FROM users",
        [IsNull::NotNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // The qualified reference can only come from the outer SELECT.
    check_infer(
        "CREATE VIEW test AS SELECT users.id, (SELECT count(users.id)) FROM users WHERE 0",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // and so can an unqualified one in a subquery without a FROM clause
    check_infer(
        "CREATE VIEW test AS SELECT users.id, (SELECT count(id)) FROM users WHERE 0",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT users.id, (SELECT max(p.id) FROM posts p) FROM users",
        [IsNull::NotNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT users.id, \
         (SELECT count(*) FILTER (WHERE p.id > 0) FROM posts p) FROM users",
        [IsNull::NotNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT users.id, \
         (SELECT count(*) FILTER (WHERE users.id > 0) FROM posts p) \
         FROM users WHERE 0",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT users.id, \
         (SELECT max(posts.id) FROM posts) FROM users",
        [IsNull::NotNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // An unqualified reference in a subquery could belong to either query, so such
    // definitions are rejected
    assert!(
        diesel_infer_query::parse_view_def(
            "CREATE VIEW test AS SELECT users.id, (SELECT max(id) FROM posts) FROM users",
            Backend::Sqlite,
        )
        .is_err()
    );
    // `t.id` refers to the argument's own subquery, which leaves `users.id` as the only
    // variable, so this is an aggregate of the outer SELECT
    check_infer(
        "CREATE VIEW test AS SELECT users.id, (SELECT count((SELECT 1 FROM posts t \
         WHERE t.id = users.id)) FROM posts t) FROM users WHERE 0",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // SQLite binds `users.name` to the outer `users` when the inner `users`, an alias of
    // `posts`, has no such column, so a name the outer SELECT also exposes, even inside a
    // parenthesized join, proves nothing
    for definition in [
        "CREATE VIEW test AS SELECT users.id, (SELECT count(users.name) FROM posts AS users) \
         FROM users WHERE 0",
        "CREATE VIEW test AS SELECT users.id, (SELECT count(USERS.name) FROM posts AS USERS) \
         FROM users WHERE 0",
        "CREATE VIEW test AS SELECT u.id, (SELECT count(u.name) FROM (SELECT id FROM posts) AS u) \
         FROM users AS u WHERE 0",
        "CREATE VIEW test AS SELECT users.id, (SELECT count(users.name) FROM posts AS users) \
         FROM (users JOIN comments ON comments.user_id = users.id) WHERE 0",
    ] {
        check_infer(
            definition,
            [IsNull::IsNullable, IsNull::IsNullable],
            [("users", "id", IsNull::NotNullable)],
        );
    }
}

#[test]
fn unknown_functions_can_be_aggregates() {
    // SQLite applications and MariaDB users can define aggregates, which make a query
    // without GROUP BY return a row with NULL for its bare columns for an empty table
    for backend in [Backend::Sqlite, Backend::Mysql, Backend::Mariadb] {
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT id, my_aggregate(id) FROM users",
            [IsNull::IsNullable, IsNull::IsNullable],
            [("users", "id", IsNull::NotNullable)],
        );
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT id, LOWER(name), coalesce(name, id) FROM users",
            [IsNull::NotNullable, IsNull::IsNullable, IsNull::NotNullable],
            [
                ("users", "id", IsNull::NotNullable),
                ("users", "name", IsNull::NotNullable),
            ],
        );
    }
    // only the built-in functions of the backend itself are known to be scalar, with the
    // number of arguments SQLite defines them for
    for (backend, definition, expected) in [
        (
            Backend::Sqlite,
            "SELECT id, aes_encrypt(id) FROM users",
            IsNull::IsNullable,
        ),
        (
            Backend::Mysql,
            "SELECT id, aes_encrypt(id) FROM users",
            IsNull::NotNullable,
        ),
        (
            Backend::Mariadb,
            "SELECT id, aes_encrypt(id) FROM users",
            IsNull::NotNullable,
        ),
        (
            Backend::Sqlite,
            "SELECT id, lower(id, id) FROM users",
            IsNull::IsNullable,
        ),
        (
            Backend::Sqlite,
            "SELECT id, round(id, 1, 2) FROM users",
            IsNull::IsNullable,
        ),
        (
            Backend::Sqlite,
            "SELECT id, round(id, 1) FROM users",
            IsNull::NotNullable,
        ),
        (
            Backend::Sqlite,
            "SELECT id, date() FROM users",
            IsNull::NotNullable,
        ),
        (
            Backend::Mysql,
            "SELECT id, ST_Collect(id) FROM users",
            IsNull::IsNullable,
        ),
    ] {
        let definition = format!("CREATE VIEW test AS {definition}").leak();
        check_infer_for(
            backend,
            definition,
            [expected, IsNull::IsNullable],
            [("users", "id", IsNull::NotNullable)],
        );
    }
    // SQLite's CURRENT_TIMESTAMP is a scalar function called without parentheses
    check_infer(
        "CREATE VIEW test AS SELECT id, CURRENT_TIMESTAMP IS NULL FROM users",
        [IsNull::NotNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // a qualified name refers to a stored function, which MariaDB allows to aggregate
    check_infer_for(
        Backend::Mariadb,
        "CREATE VIEW test AS SELECT id, db.lower(id) FROM users",
        [IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // PostgreSQL rejects a column outside the aggregates without GROUP BY, so a view with
    // one calls no aggregate, whatever sqlparser reads as a call, like the ARRAY
    // constructor of a subquery
    check_infer_for(
        Backend::Pg,
        "CREATE VIEW test AS SELECT id, my_function(id), ARRAY(SELECT 1) FROM users",
        [IsNull::NotNullable, IsNull::IsNullable, IsNull::Unknown],
        [("users", "id", IsNull::NotNullable)],
    );
}

#[test]
fn aggregate_rows_reach_every_query() {
    // the arguments of `coalesce()` decide its nullability, so they are bare columns too
    check_infer(
        "CREATE VIEW test AS SELECT coalesce(id, name), count(*) FROM users",
        [IsNull::IsNullable, IsNull::NotNullable],
        [
            ("users", "id", IsNull::NotNullable),
            ("users", "name", IsNull::NotNullable),
        ],
    );
    // common table expressions, derived tables, and subqueries can aggregate as well
    check_infer(
        "CREATE VIEW test AS WITH c AS (SELECT id, count(*) AS n FROM users) \
         SELECT c.id, d.id, 1 IN (SELECT e.id FROM users AS e HAVING count(*) >= 0) \
         FROM c, (SELECT id, max(id) AS m FROM users) AS d",
        [IsNull::IsNullable, IsNull::IsNullable, IsNull::IsNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    // MySQL and MariaDB add super-aggregate rows WITH ROLLUP
    for backend in [Backend::Mysql, Backend::Mariadb] {
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT id, count(*) FROM users GROUP BY id WITH ROLLUP",
            [IsNull::IsNullable, IsNull::NotNullable],
            [("users", "id", IsNull::NotNullable)],
        );
    }
}

#[test]
fn relations_resolve_in_their_scope() {
    // `users` of the common table expression inside `c` is out of scope for the outer
    // query, which reads the table
    check_infer(
        "CREATE VIEW test AS WITH c AS (WITH users AS (SELECT 'x' AS hair_color) \
         SELECT hair_color FROM users) SELECT users.hair_color FROM users",
        [IsNull::IsNullable],
        [("users", "hair_color", IsNull::IsNullable)],
    );
    // derived tables in the arms of a set operation and inside other derived tables
    check_infer(
        "CREATE VIEW test AS SELECT d.x FROM (SELECT 1 AS x) AS d \
         UNION SELECT e.x FROM (SELECT f.x FROM (SELECT NULL AS x) AS f) AS e",
        [IsNull::IsNullable],
        (),
    );
    check_infer(
        "CREATE VIEW test AS SELECT d.x FROM (SELECT 1 AS x) AS d \
         UNION SELECT e.x FROM (SELECT f.x FROM (SELECT 2 AS x) AS f) AS e",
        [IsNull::NotNullable],
        (),
    );
    // derived tables inside a parenthesized join, and next to an unnamed one
    check_infer(
        "CREATE VIEW test AS SELECT d.x, users.id \
         FROM (users JOIN (SELECT 1 AS x) AS d ON d.x = users.id)",
        [IsNull::NotNullable, IsNull::NotNullable],
        [("users", "id", IsNull::NotNullable)],
    );
    check_infer(
        "CREATE VIEW test AS SELECT d.x FROM (SELECT 1 AS x) AS d, (SELECT NULL AS y)",
        [IsNull::NotNullable],
        (),
    );
    // the resolver only knows the relations around a subquery
    for definition in [
        "CREATE VIEW test AS SELECT 1 IN (WITH users AS (SELECT NULL AS id) \
         SELECT id FROM users) FROM users",
        "CREATE VIEW test AS SELECT 1 IN (SELECT d.id FROM (SELECT NULL AS id) AS d) \
         FROM users",
    ] {
        check_infer(definition, [IsNull::Unknown], ());
    }
    // a recursive common table expression reads itself, not the table `t`
    check_infer(
        "CREATE VIEW test AS WITH RECURSIVE t(n, m) AS (SELECT 1, NULL \
         UNION ALL SELECT t.m, t.m FROM t WHERE t.n IS NOT NULL) SELECT n FROM t",
        [IsNull::Unknown],
        [
            ("t", "n", IsNull::NotNullable),
            ("t", "m", IsNull::NotNullable),
        ],
    );
}

#[test]
fn cte_column_lists_match_their_queries() {
    // SQLite rejects a column list longer or shorter than the query
    for definition in [
        "CREATE VIEW test AS WITH c(a) AS (SELECT 1, 2) SELECT c.a FROM c",
        "CREATE VIEW test AS WITH c(a, b) AS (SELECT 1) SELECT c.a FROM c",
    ] {
        let res = diesel_infer_query::parse_view_def(definition, Backend::Sqlite);
        assert!(
            matches!(res, Err(diesel_infer_query::Error::UnsupportedSql { .. })),
            "{definition}: {res:?}"
        );
    }
}

#[test]
fn ambiguous_relation_names() {
    for definition in [
        // SQLite reads the common table expression `b`, PostgreSQL and MySQL the table
        "CREATE VIEW test AS WITH a AS (SELECT id FROM b), b AS (SELECT NULL AS id) \
         SELECT id FROM a",
        // one query defines `d` twice
        "CREATE VIEW test AS SELECT d.x FROM (SELECT 1 AS x) AS d \
         UNION SELECT d.x FROM (SELECT NULL AS x) AS d",
        // the subquery reads the table `users`, not the derived table
        "CREATE VIEW test AS SELECT 1 IN (SELECT id FROM users) \
         FROM (SELECT NULL AS id) AS users",
        // two unnamed derived tables, which the resolver cannot tell apart
        "CREATE VIEW test AS SELECT x FROM (SELECT 1 AS x), (SELECT NULL AS y)",
    ] {
        let res = diesel_infer_query::parse_view_def(definition, Backend::Sqlite);
        assert!(
            matches!(res, Err(diesel_infer_query::Error::UnsupportedSql { .. })),
            "{definition}: {res:?}"
        );
    }
    // `c` inside `d` matches the inner `C` only ignoring case, but the outer `c` exactly
    let mut resolver = Resolver::from(());
    let mut view_def = diesel_infer_query::parse_view_def(
        "CREATE VIEW test AS WITH c AS (SELECT 1 AS x), \
         d AS (WITH C AS (SELECT NULL AS x) SELECT c.x FROM c) SELECT d.x FROM d",
        Backend::Sqlite,
    )
    .unwrap();
    view_def.resolve_references(&mut resolver).unwrap();
    let res = view_def.infer_nullability(&mut resolver);
    assert!(
        matches!(res, Err(diesel_infer_query::Error::ResolverFailure { .. })),
        "{res:?}"
    );
}

const BACKENDS: [Backend; 4] = [
    Backend::Sqlite,
    Backend::Pg,
    Backend::Mysql,
    Backend::Mariadb,
];

/// The nullability `nullable` stands for
fn is_null(nullable: bool) -> IsNull {
    if nullable {
        IsNull::IsNullable
    } else {
        IsNull::NotNullable
    }
}

#[test]
fn casts_that_can_return_null() {
    // MySQL and MariaDB return NULL for a value the target type cannot hold, like
    // `CAST('0000-00-00' AS DATE)` or MariaDB's `CAST('x' AS INET6)`, and TRY_CAST
    // returns NULL when the conversion fails
    for backend in BACKENDS {
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT CAST('0000-00-00' AS DATE), CAST('x' AS DATETIME), \
                  CAST('x' AS TIME), CAST(99999 AS YEAR), CAST('x' AS INET6), \
                  TRY_CAST('x' AS INTEGER), CAST('x' AS INTEGER), CAST(1 AS TEXT), \
                  CAST('x' AS DECIMAL(5, 2)), CAST('x' AS REAL), CAST('x' AS BLOB)",
            [
                IsNull::IsNullable,
                IsNull::IsNullable,
                IsNull::IsNullable,
                IsNull::IsNullable,
                IsNull::IsNullable,
                IsNull::IsNullable,
                IsNull::NotNullable,
                IsNull::NotNullable,
                IsNull::NotNullable,
                IsNull::NotNullable,
                IsNull::NotNullable,
            ],
            (),
        );
    }
    // PostgreSQL 18 casts a JSON null to NULL when converting to a number or a boolean,
    // so such casts are only non-null for a literal, which cannot be JSON. MySQL returns
    // NULL for bytes that are no valid string, and MariaDB for a result longer than its
    // max_allowed_packet.
    for (backend, expected) in [
        (Backend::Sqlite, [true, true, true, false, false]),
        (Backend::Pg, [true, true, true, false, false]),
        (Backend::Mysql, [true, true, true, true, false]),
        (Backend::Mariadb, [true, true, true, true, true]),
    ] {
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT CAST(id AS INTEGER), CAST(id AS BOOLEAN), \
                  CAST(id AS REAL), CAST(name AS TEXT), CAST(name AS BLOB) FROM users",
            expected.map(is_null),
            [
                ("users", "id", IsNull::NotNullable),
                ("users", "name", IsNull::NotNullable),
            ],
        );
    }
    // SQLite accepts these names of string and byte types, and a cast never turns a
    // non-null value into NULL
    check_infer(
        "CREATE VIEW test AS SELECT CAST(name AS CHARACTER(10)), \
         CAST(name AS CHAR VARYING(10)), CAST(name AS NVARCHAR(10)), \
         CAST(name AS VARBINARY(10)), CAST(name AS CLOB), \
         CAST(name AS CHARACTER LARGE OBJECT), CAST(name AS CHAR LARGE OBJECT), \
         CAST(name AS STRING) FROM users",
        [IsNull::NotNullable; 8],
        [("users", "name", IsNull::NotNullable)],
    );
}

#[test]
fn mysql_and_mariadb_casts_to_strings_and_bytes() {
    // MySQL returns NULL for a declared length above its max_allowed_packet, which is at
    // least 1024, and in strict mode for bytes that are no valid string of the target
    // character set. MariaDB caps the declared length instead, but returns NULL for a
    // result longer than max_allowed_packet in bytes, and keeps up to the declared number
    // of characters of the operand even for bytes, where a character takes up to 4.
    let definition = "CREATE VIEW test AS SELECT CAST('x' AS CHAR(1024)), \
        CAST('x' AS CHAR(1025)), CAST(1 AS CHAR(1025)), CAST('x' AS BINARY(1000000000)), \
        CAST(name AS CHAR(256)), CAST(name AS CHAR(257)), CAST(name AS BINARY(1024)), \
        CAST(name AS BINARY(1025)), CAST(name AS BINARY), CAST(x'FF' AS CHAR), \
        CAST(x'FF' AS BINARY) FROM users";
    for (backend, expected) in [
        (
            Backend::Mysql,
            [
                false, true, true, true, true, true, false, true, false, true, false,
            ],
        ),
        (
            Backend::Mariadb,
            [
                false, false, false, false, false, true, true, true, true, true, true,
            ],
        ),
        // SQLite and PostgreSQL turn any value into a string or bytes
        (Backend::Sqlite, [false; 11]),
        (Backend::Pg, [false; 11]),
    ] {
        check_infer_for(
            backend,
            definition,
            expected.map(is_null),
            [("users", "name", IsNull::NotNullable)],
        );
    }
    // double-quoted and national strings are literals too
    for backend in [Backend::Mysql, Backend::Mariadb] {
        check_infer_for(
            backend,
            "CREATE VIEW test AS SELECT CAST(\"x\" AS CHAR(1024)), CAST(N'x' AS CHAR(1024))",
            [IsNull::NotNullable, IsNull::NotNullable],
            (),
        );
    }
}
