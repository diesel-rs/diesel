// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel_infer_query::{Backend, Error, parse_view_def};

#[track_caller]
pub(crate) fn check_parse_view(name: &'static str, def: &'static str) {
    let res = parse_view_def(def, Backend::Sqlite);
    assert!(
        res.is_ok(),
        "Failed to infer SQL with error: {}",
        res.unwrap_err()
    );
    let res = res.unwrap();

    let mut settions = insta::Settings::new();
    settions.set_raw_info(&(def.into()));
    settions.bind(|| {
        insta::assert_debug_snapshot!(name, res);
    });
}

#[test]
pub(crate) fn view_with_literals() {
    check_parse_view(
        "view_with_literals",
        "CREATE VIEW test AS SELECT 1 AS foo, 'foo' AS bar, NULL",
    );
}

#[test]
pub(crate) fn simple_table() {
    check_parse_view(
        "simple_table",
        "CREATE VIEW test AS SELECT users.id, name, hair_color as hair_colour FROM users",
    );
}

#[test]
pub(crate) fn simple_table_with_alias() {
    check_parse_view(
        "simple_table_with_alias",
        "CREATE VIEW test AS SELECT u.id, name, u.hair_color AS hair_colour FROM users as u",
    );
}

#[test]
pub(crate) fn using_ops() {
    check_parse_view(
        "using_ops",
        "CREATE VIEW ops AS SELECT 1 + 2, json @> 'json', name IS NULL FROM bar",
    );
}

#[test]
pub(crate) fn using_function() {
    check_parse_view(
        "using_functions",
        "CREATE VIEW ops AS SELECT count(*), sum(foo) FROM bar",
    );
}

#[test]
pub(crate) fn is_null_and_not_null() {
    check_parse_view(
        "is_null_and_not_null",
        "CREATE VIEW test AS SELECT 1 IS NOT NULL, 2 IS NULL",
    );
}

#[test]
fn definitions_are_read_in_the_dialect_of_their_backend() {
    // the SQLite dialect cannot read PostgreSQL's `?`, which checks for a key of a JSON
    // object, or MySQL's `DIV`, which divides integers
    for (backend, definition) in [
        (
            Backend::Pg,
            "CREATE VIEW test AS SELECT doc ? 'k' FROM users",
        ),
        (Backend::Mysql, "CREATE VIEW test AS SELECT 7 DIV 2"),
        (Backend::Mariadb, "CREATE VIEW test AS SELECT 7 DIV 2"),
    ] {
        let res = parse_view_def(definition, backend);
        assert!(res.is_ok(), "{backend:?}: {res:?}");
    }
}

#[test]
fn wildcard_select() {
    check_parse_view("wildcard_select", "CREATE VIEW test AS SELECT * FROM users");
}

#[test]
fn qualified_wildcard_select() {
    check_parse_view(
        "qualified_wildcard_select",
        "CREATE VIEW test AS SELECT users.* FROM users",
    );
}

#[test]
fn qualified_wildcard_select_left_join() {
    check_parse_view(
        "qualified_wildcard_select_left_join",
        "CREATE VIEW test AS SELECT users.*, posts.* FROM users LEFT JOIN posts ON users.id = posts.user_id",
    );
}

#[test]
fn is_distinct_from() {
    check_parse_view(
        "is_distinct_from",
        "CREATE VIEW test AS SELECT 'abc' IS DISTINCT FROM NULL, 'def' IS NOT DISTINCT FROM NULL",
    )
}

#[test]

fn like() {
    check_parse_view(
        "like",
        "CREATE VIEW test AS SELECT 'abc' LIKE 'foo', 'cde' LIKE NULL, \
              'fgh' ILIKE '%', 'ijk' ILIKE NULL, 'abc' NOT LIKE '%', 'abc' NOT LIKE NULL",
    );
}

#[test]
fn between() {
    check_parse_view(
        "between",
        "CREATE VIEW test AS SELECT 1 BETWEEN 0 AND 10, 1 BETWEEN NULL AND 25, 1 NOT BETWEEN 0 AND 10, 1 NOT BETWEEN NULL AND 25",
    )
}

#[test]
fn similar_to() {
    check_parse_view(
        "similar_to",
        "CREATE VIEW test AS SELECT 'abc' SIMILAR TO 'cde', 'ABC' NOT SIMILAR TO NULL, NULL SIMILAR TO 'abc'",
    )
}

#[test]
fn regexp() {
    // TODO: not supported by the parser?
    // also 'abc' RLIKE NULL is not supported
    check_parse_view(
        "regexp",
        "CREATE VIEW test AS SELECT 'abc' REGEXP 'abc', NULL REGEXP 'abc', 'abc' REGEXP NULL,\
        'abc' RLIKE 'abc', NULL RLIKE 'abc'",
    )
}

#[test]
fn case_when() {
    check_parse_view(
        "case_when",
        "CREATE VIEW test AS SELECT \
              CASE WHEN 1 = 1 THEN 1 WHEN NULL THEN 1 ELSE 1 END,
              CASE WHEN 1 = 1 THEN 1 WHEN NULL THEN 1 ELSE NULL END,
              CASE WHEN 1 = 1 THEN NULL WHEN NULL THEN 1 ELSE 1 END",
    );
}

#[test]
fn in_list() {
    check_parse_view(
        "in_list",
        "CREATE VIEW test AS SELECT name IN (1, 2, 4) FROM users",
    );
}

#[test]
fn not_in_list() {
    check_parse_view(
        "not_in_list",
        "CREATE VIEW test AS SELECT name NOT IN (1, 2, 4) FROM users",
    );
}

#[test]
fn in_subquery() {
    check_parse_view(
        "in_subquery",
        "CREATE VIEW test AS SELECT name IN (SELECT posts.name FROM posts) FROM users",
    );
}

#[test]
fn not_in_subquery() {
    check_parse_view(
        "not_in_subquery",
        "CREATE VIEW test AS SELECT name NOT IN (SELECT posts.name FROM posts) FROM users",
    );
}

#[test]
fn nested() {
    check_parse_view("nested", "CREATE VIEW test AS SELECT (1 + 2) - 3");
}

#[test]
fn subquery() {
    check_parse_view(
        "subquery",
        "CREATE VIEW test AS SELECT \
                  (SELECT users.name FROM users WHERE users.id = posts.user_id), \
                  (SELECT users.hair_color FROM users WHERE users.id = posts.user_id), \
                  (SELECT posts.body FROM users WHERE users.id = posts.user_id)
              FROM posts",
    );
}

#[test]
fn with_cte() {
    check_parse_view(
        "with_cte",
        "CREATE VIEW test AS WITH source AS (\
             SELECT a, b FROM table1 WHERE c > 1\
         ) SELECT t1.col1, s.b, count(*) FROM users t1 \
         INNER JOIN source s ON t1.id = s.a GROUP BY t1.col1, s.b",
    );
}

#[test]
fn with_multi_level_cte() {
    check_parse_view(
        "with_multi_level_cte",
        "CREATE VIEW test AS WITH source AS (\
             SELECT a, b FROM table1 WHERE c > 1\
             ), \
             intermediate AS (SELECT a + b AS one, a, b, count(*) as c FROM source) \
         SELECT t1.col1, s.a, s.b, count(*), s.one, s.c FROM users t1 \
         INNER JOIN intermediate s ON t1.id = s.a GROUP BY t1.col1, s.b",
    )
}

#[test]
fn from_subquery() {
    check_parse_view(
        "from_subquery",
        "CREATE VIEW test AS SELECT t1.some_col, users.name FROM (\
         SELECT some_col FROM posts WHERE posts.id < 10) AS t1 \
         INNER JOIN users ON users.id = t1.some_col",
    );
}

#[test]
fn window_functions() {
    check_parse_view(
        "window_functions",
        "CREATE VIEW test AS SELECT user_id, name, \
         row_number() OVER (PARTITION BY department ORDER BY salary DESC) as rn \
         FROM employees",
    );
}

#[test]
fn casting() {
    check_parse_view(
        "casting",
        "CREATE VIEW test AS SELECT '123'::integer AS id, CAST(price AS text), name::varchar AS v_name FROM items",
    );
}

#[test]
fn recursive_cte() {
    check_parse_view(
        "recursive_cte",
        "CREATE VIEW test AS WITH RECURSIVE cte (id, name, parent_id) AS (\
               SELECT employee_id, employee_name, parent_id FROM employees WHERE parent_id IS NULL \
               UNION ALL \
               SELECT e.employee_id, e.employee_name, e.parent_id FROM employees e JOIN cte ON e.parent_id = cte.id \
           ) SELECT id, name, parent_id FROM cte",
    );
}

#[test]
fn complex_cte_subquery_set_ops() {
    check_parse_view(
           "complex_cte_subquery_set_ops",
           "CREATE VIEW test AS WITH \
               stage1 AS (SELECT a, b, c, d FROM sources WHERE c > 1), \
               stage2 AS (SELECT a, b, c, d FROM other_sources WHERE c > 1), \
               union_set AS (SELECT a, b, c, d FROM stage1 UNION SELECT a, b, c, d FROM stage2), \
               cte_nested AS (
                   SELECT t.a, t.b, t.c, t.d, s.col1, s.col2 FROM union_set t JOIN (SELECT col1, col2 FROM small_table LIMIT 1) s ON t.a = s.col
               ) \
           SELECT a, b, c, d, col1, col2 FROM cte_nested",
       );
}

#[test]
fn union() {
    check_parse_view(
        "union",
        "CREATE VIEW test AS SELECT id, name FROM old_users WHERE is_active = TRUE \
         UNION \
         SELECT user_id, display_name FROM new_members",
    );
}

#[test]
fn intersect() {
    check_parse_view(
        "intersect",
        "CREATE VIEW test AS SELECT project_id, user_id FROM assigned_tasks WHERE status = 'COMPLETED' \
            INTERSECT \
            SELECT product_id, associated_user FROM purchases WHERE purchased = TRUE",
    );
}

#[test]
fn except() {
    check_parse_view(
        "except",
        "CREATE VIEW test AS SELECT owner_id, file_hash FROM content_vault WHERE created < '2026-08-07' \
         EXCEPT \
         SELECT creator_id, sha256 FROM recent_uploads",
    );
}

#[test]
fn unnamed_query_source() {
    check_parse_view("unnamed_query_source", "SELECT * FROM (SELECT 1 as dummy)");
}

#[test]
fn unknown_qualified_wildcard() {
    // the same error as an unknown qualifier of a column
    for definition in [
        "CREATE VIEW test AS SELECT posts.* FROM users",
        "CREATE VIEW test AS SELECT posts.id FROM users",
    ] {
        let res = parse_view_def(definition, Backend::Sqlite);
        assert!(
            matches!(&res, Err(Error::InvalidQuerySource { query_source, .. }) if query_source == "posts"),
            "{definition}: {res:?}"
        );
    }
}

#[test]
fn cyclic_join() {
    // `b` is joined through `c`, and `c` through `b`
    let res = parse_view_def(
        "CREATE VIEW test AS SELECT b.* FROM a JOIN b ON b.id = c.id JOIN c ON c.id = b.id",
        Backend::Sqlite,
    );
    assert!(matches!(res, Err(Error::UnsupportedSql { .. })), "{res:?}");
    // `b` is joined through `z`, which the FROM clause lacks
    let res = parse_view_def(
        "CREATE VIEW test AS SELECT b.* FROM a JOIN b ON b.id = z.id",
        Backend::Sqlite,
    );
    assert!(
        matches!(&res, Err(Error::InvalidQuerySource { query_source, .. }) if query_source == "z"),
        "{res:?}"
    );
}

#[test]
fn partially_parsed_definition() {
    // sqlparser cannot parse `<<` in SQLite views, so it gives up on the CASE, reads
    // `CASE (1)` as a call of a function named `CASE`, and stops before `WHEN 1`. That
    // dropped the rest of the query, including its FROM clause.
    for definition in [
        "CREATE VIEW test AS SELECT CASE (1) WHEN 1 THEN id << 1 END FROM users",
        "SELECT id FROM users; SELECT name FROM users",
    ] {
        let res = parse_view_def(definition, Backend::Sqlite);
        assert!(res.is_err(), "{definition}: {res:?}");
    }
    // PostgreSQL ends its view definitions with a semicolon
    let res = parse_view_def(" SELECT users.id FROM users;", Backend::Pg);
    assert!(res.is_ok(), "{res:?}");
}
