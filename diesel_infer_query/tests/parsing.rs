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
