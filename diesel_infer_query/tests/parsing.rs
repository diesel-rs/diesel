// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel_infer_query::parse_view_def;

#[track_caller]
pub(crate) fn check_parse_view(name: &'static str, def: &'static str) {
    let res = parse_view_def(def);
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
