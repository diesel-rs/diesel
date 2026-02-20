#[cfg(diesel_docsrs)]
fn inner_format(input: String, expanded: String) -> String {
    let input = input.trim();
    let expanded = expanded.trim();
    format!(
        r#"

#### Input

```rust,ignore
{input}
```

#### Expanded Code

<div class="warning">Expanded code might use diesel internal API's and is only shown for educational purpose</div>

The macro expands the input to the following Rust code:


```rust,ignore
{expanded}
```
"#
    )
}

#[cfg(diesel_docsrs)]
fn normal_format(input: String, expanded: String) -> String {
    let doc = inner_format(input, expanded);
    write_detail_section(doc)
}

#[cfg(diesel_docsrs)]
fn write_detail_section(content: String) -> String {
    format!(
        r#"
# Expanded Code

<details>
<summary> Expanded Code </summary>

{content}

</details>
"#
    )
}

#[cfg(diesel_docsrs)]
fn read_snapshot(snapshot_dir: &std::path::Path, file: &str) -> (String, String) {
    let file = snapshot_dir.join(file);
    let content = std::fs::read_to_string(&file)
        .expect(&format!("Failed to read snapshot: `{}`", file.display()));
    let mut lines = content
        .lines()
        .skip_while(|l| !l.trim().starts_with("input:"));
    let input = lines.next().expect("input field exists");
    let input = input.trim().strip_prefix("input:").unwrap_or(input).trim();
    let input = input.strip_prefix("\"").unwrap_or(input);
    let input = input.strip_suffix("\"").unwrap_or(input);
    let input = input.replace("\\n", "\n").replace("\\\"", "\"");

    let lines = lines.skip_while(|l| *l != "---").skip(1);
    let content = lines.collect::<Vec<_>>().join("\n");

    (input, content)
}

#[cfg(diesel_docsrs)]
fn write_multiple_part(
    snapshot_dir: &std::path::Path,
    file: &str,
    heading: &str,
    out: &mut String,
) {
    use std::fmt::Write;
    let (input, content) = read_snapshot(&snapshot_dir, file);
    writeln!(out).expect("This doesn't fail");
    writeln!(out, "### {heading}").expect("This doesn't fail");
    writeln!(out).expect("This doesn't fail");
    let doc = inner_format(input, content);
    writeln!(out, "{doc}").expect("This doesn't fail");
}

#[cfg(diesel_docsrs)]
fn format_multiple(snapshot_dir: &std::path::Path, block: &[Example]) -> String {
    // sql_type is special as it depends on all feature flags
    // so we have a custom block here:
    let mut doc = String::new();
    for example in block {
        write_multiple_part(&snapshot_dir, example.snapshot, example.heading, &mut doc);
    }
    if !doc.is_empty() {
        doc = write_detail_section(doc);
    }
    doc
}
#[cfg(diesel_docsrs)]
struct Example {
    snapshot: &'static str,
    heading: &'static str,
}
#[cfg(diesel_docsrs)]
impl Example {
    const fn new(snapshot: &'static str) -> Self {
        Self {
            snapshot,
            heading: "",
        }
    }
    const fn with_heading(snapshot: &'static str, heading: &'static str) -> Self {
        Self { snapshot, heading }
    }
}

#[cfg(diesel_docsrs)]
fn main() {
    use std::path::PathBuf;

    let snapshot_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join("snapshots");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let has_sqlite = std::env::var("CARGO_FEATURE_SQLITE").is_ok();
    let has_postgres = std::env::var("CARGO_FEATURE_POSTGRES").is_ok();
    let has_mysql = std::env::var("CARGO_FEATURE_MYSQL").is_ok();

    let mut mapping = [
        (
            "allow_tables_to_appear_in_same_query",
            vec![
                Example::with_heading("diesel_derives__tests__simple.snap", "Simple example"),
                Example::with_heading("diesel_derives__tests__with_paths.snap", "With paths"),
            ],
        ),
        (
            "as_changeset",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_treat_none_as_null_1.snap",
                    "With `#[diesel(treat_none_as_null = true)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_primary_key_1.snap",
                    "With `#[diesel(primary_key(id, short_code))]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_table_name_1.snap",
                    "With `#[diesel(table_name = crate::schema::users)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_change_field_type_1.snap",
                    "With `#[serialize_as = String]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_embed_1.snap",
                    "With `#[diesel(embed)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_column_name_1.snap",
                    "With `#[diesel(column_name = username)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_treat_none_field_as_null_1.snap",
                    "With `#[diesel(treat_none_as_null = true)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_changeset_treat_skip_update_1.snap",
                    "With `#[diesel(skip_update)]`",
                ),
            ],
        ),
        (
            "as_expression",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__as_expression_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__as_expression_not_sized_1.snap",
                    "With `#[diesel(not_sized)]`",
                ),
            ],
        ),
        (
            "associations",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__associations_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__associations_table_name_1.snap",
                    "With `#[diesel(table_name = crate::schema::posts)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__associations_column_name_1.snap",
                    "With `#[diesel(column_name = author_id)]`",
                ),
            ],
        ),
        (
            "auto_type",
            vec![Example::new("diesel_derives__tests__auto_type_1.snap")],
        ),
        (
            "declare_sql_function",
            vec![
                Example::with_heading(
                    if has_sqlite {
                        "diesel_derives__tests__declare_sql_function_1 (sqlite).snap"
                    } else {
                        "diesel_derives__tests__declare_sql_function_1.snap"
                    },
                    "Without attributes",
                ),
                Example::with_heading(
                    if has_sqlite {
                        "diesel_derives__tests__declare_sql_function_aggregate_1 (sqlite).snap"
                    } else {
                        "diesel_derives__tests__declare_sql_function_aggregate_1.snap"
                    },
                    "With `#[aggregate]`",
                ),
                Example::with_heading(
                    if has_sqlite {
                        "diesel_derives__tests__declare_sql_function_sql_name_1 (sqlite).snap"
                    } else {
                        "diesel_derives__tests__declare_sql_function_sql_name_1.snap"
                    },
                    "With `#[sql_name = \"MY_LOWER\"]`",
                ),
                Example::with_heading(
                    if has_sqlite {
                        "diesel_derives__tests__declare_sql_function_window_1 (sqlite).snap"
                    } else {
                        "diesel_derives__tests__declare_sql_function_window_1.snap"
                    },
                    "With `#[window]`",
                ),
                Example::with_heading(
                    if has_sqlite {
                        "diesel_derives__tests__declare_sql_function_variadic_1 (sqlite).snap"
                    } else {
                        "diesel_derives__tests__declare_sql_function_variadic_1.snap"
                    },
                    "With `#[variadic(argument_count)]`",
                ),
            ],
        ),
        (
            "define_sql_function",
            vec![Example::new(if has_sqlite {
                "diesel_derives__tests__define_sql_function_1 (sqlite).snap"
            } else {
                "diesel_derives__tests__define_sql_function_1.snap"
            })],
        ),
        (
            "from_sql_row",
            vec![Example::new("diesel_derives__tests__from_sql_row_1.snap")],
        ),
        (
            "identifiable",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__identifiable_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__identifiable_table_name_1.snap",
                    "With `#[diesel(table_name = crate::schema::admin_users)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__identifiable_primary_key_1.snap",
                    "With `#[diesel(primary_key(id, short_code))]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__identifiable_column_name_1.snap",
                    "With `#[diesel(column_name = user_id)]`",
                ),
            ],
        ),
        (
            "insertable",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__insertable_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__insertable_table_name_1.snap",
                    "With `#[diesel(table_name = crate::schema::admin_users)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__insertable_treat_none_as_default_value_1.snap",
                    "With `#[diesel(treat_none_as_default_value = false)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__insertable_column_name_1.snap",
                    "With `#[diesel(column_name = username)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__insertable_embed_1.snap",
                    "With `#[diesel(embed)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__insertable_serialize_as_1.snap",
                    "With `#[diesel(serialize_as = String)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__insertable_treat_none_as_default_value_field_1.snap",
                    "With `#[diesel(treat_none_as_default_value = true)]` on field",
                ),
                Example::with_heading(
                    "diesel_derives__tests__insertable_skip_insertion_1.snap",
                    "With `#[diesel(skip_insertion)]`",
                ),
            ],
        ),
        (
            "multiconnection",
            vec![Example::new(
                "diesel_derives__tests__multiconnection_1.snap",
            )],
        ),
        (
            "queryable",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__queryable_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__queryable_deserialize_as_1.snap",
                    "With `#[diesel(deserialize_as = String)]`",
                ),
            ],
        ),
        (
            "queryable_by_name",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__queryable_by_name_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__queryable_by_name_table_name_1.snap",
                    "With `#[diesel(table_name = crate::schema::users)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__queryable_by_name_check_for_backend_1.snap",
                    "With `#[diesel(check_for_backend(diesel::pg::Pg, diesel::mysql::Mysql))]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__queryable_by_name_column_name_1.snap",
                    "With `#[diesel(column_name = username)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__queryable_by_name_sql_type_1.snap",
                    "With `#[diesel(sql_type = diesel::sql_types::Text)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__queryable_by_name_deserialize_as_1.snap",
                    "With `#[diesel(deserialize_as = String)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__queryable_by_name_embed_1.snap",
                    "With `#[diesel(embed)]`",
                ),
            ],
        ),
        (
            "query_id",
            vec![Example::new("diesel_derives__tests__query_id_1.snap")],
        ),
        (
            "selectable",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__selectable_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__selectable_check_for_backend_1.snap",
                    "With `#[diesel(check_for_backend(diesel::pg::Pg, diesel::mysql::Mysql))]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__selectable_column_name_1.snap",
                    "With `#[diesel(column_name = username)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__selectable_embed_1.snap",
                    "With `#[diesel(embed)]`",
                ),
                Example::with_heading(
                    "diesel_derives__tests__selectable_select_expression_1.snap",
                    "With `#[diesel(select_expression = ...)]` and `#[diesel(select_expression_type = ...)]`",
                ),
            ],
        ),
        (
            "table",
            vec![Example::new("diesel_derives__tests__table_1.snap")],
        ),
        (
            "view",
            vec![Example::new("diesel_derives__tests__view_1.snap")],
        ),
        (
            "valid_grouping",
            vec![
                Example::with_heading(
                    "diesel_derives__tests__valid_grouping_1.snap",
                    "Without attributes",
                ),
                Example::with_heading(
                    "diesel_derives__tests__valid_grouping_aggregate_1.snap",
                    "With `#[diesel(aggregate)]`",
                ),
            ],
        ),
        ("sql_type", vec![]),
        ("has_query", vec![]),
    ];

    if has_sqlite {
        mapping[mapping.len() - 2].1.push(Example::with_heading(
            "diesel_derives__tests__sql_type_1 (sqlite).snap",
            "SQLite",
        ));
    }
    if has_postgres {
        mapping[mapping.len() - 2].1.push(Example::with_heading(
            "diesel_derives__tests__sql_type_1 (postgres).snap",
            "PostgreSQL",
        ));
    }
    if has_mysql {
        mapping[mapping.len() - 2].1.push(Example::with_heading(
            "diesel_derives__tests__sql_type_1 (mysql).snap",
            "MySQL",
        ));
    }

    {
        let has_query = &mut mapping[mapping.len() - 1].1;
        if has_sqlite {
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_1 (sqlite).snap",
                "Without attributes (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_base_query_1 (sqlite).snap",
                "With `#[diesel(base_query = ...)]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_base_query_type_1 (sqlite).snap",
                "With `#[diesel(base_query_type = ...)]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_table_name_1 (sqlite).snap",
                "With `#[diesel(table_name = ...)]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_check_for_backend_1 (sqlite).snap",
                "With `#[diesel(check_for_backend(...))]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_check_for_backend_disable_1 (sqlite).snap",
                "With `#[diesel(check_for_backend(disable = true))]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_column_name_1 (sqlite).snap",
                "With `#[diesel(column_name = ...)]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_embed_1 (sqlite).snap",
                "With `#[diesel(embed)]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_select_expression_1 (sqlite).snap",
                "With `#[diesel(select_expression = ...)]` (SQLite)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_deserialize_as_1 (sqlite).snap",
                "With `#[diesel(deserialize_as = ...)]` (SQLite)",
            ));
        }
        if has_postgres {
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_1 (postgres).snap",
                "Without attributes (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_base_query_1 (postgres).snap",
                "With `#[diesel(base_query = ...)]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_base_query_type_1 (postgres).snap",
                "With `#[diesel(base_query_type = ...)]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_table_name_1 (postgres).snap",
                "With `#[diesel(table_name = ...)]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_check_for_backend_1 (postgres).snap",
                "With `#[diesel(check_for_backend(...))]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_check_for_backend_disable_1 (postgres).snap",
                "With `#[diesel(check_for_backend(disable = true))]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_column_name_1 (postgres).snap",
                "With `#[diesel(column_name = ...)]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_embed_1 (postgres).snap",
                "With `#[diesel(embed)]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_select_expression_1 (postgres).snap",
                "With `#[diesel(select_expression = ...)]` (PostgreSQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_deserialize_as_1 (postgres).snap",
                "With `#[diesel(deserialize_as = ...)]` (PostgreSQL)",
            ));
        }
        if has_mysql {
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_1 (mysql).snap",
                "Without attributes (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_base_query_1 (mysql).snap",
                "With `#[diesel(base_query = ...)]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_base_query_type_1 (mysql).snap",
                "With `#[diesel(base_query_type = ...)]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_table_name_1 (mysql).snap",
                "With `#[diesel(table_name = ...)]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_check_for_backend_1 (mysql).snap",
                "With `#[diesel(check_for_backend(...))]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_check_for_backend_disable_1 (mysql).snap",
                "With `#[diesel(check_for_backend(disable = true))]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_column_name_1 (mysql).snap",
                "With `#[diesel(column_name = ...)]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_embed_1 (mysql).snap",
                "With `#[diesel(embed)]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_select_expression_1 (mysql).snap",
                "With `#[diesel(select_expression = ...)]` (MySQL)",
            ));
            has_query.push(Example::with_heading(
                "diesel_derives__tests__has_query_deserialize_as_1 (mysql).snap",
                "With `#[diesel(deserialize_as = ...)]` (MySQL)",
            ));
        }
    }

    for (derive, examples) in mapping {
        let doc = match examples.as_slice() {
            [single] => {
                let (input, content) = read_snapshot(&snapshot_dir, single.snapshot);
                normal_format(input, content)
            }
            multiple => format_multiple(&snapshot_dir, multiple),
        };
        let out_path = out.join(format!("{derive}.md"));
        std::fs::write(out_path, doc).unwrap();
    }
}

#[cfg(not(diesel_docsrs))]
fn main() {
    // just do nothing
}
