#[cfg(diesel_docsrs)]
fn inner_format(input: String, expanded: String) -> String {
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
    let content = std::fs::read_to_string(file).expect("Failed to read snapshot");
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
fn write_multiple(
    snapshot_dir: &std::path::Path,
    block: &[(&str, &str)],
    name: &str,
    out: &std::path::Path,
) {
    // sql_type is special as it depends on all feature flags
    // so we have a custom block here:
    let mut doc = String::new();
    for (heading, file) in block {
        write_multiple_part(&snapshot_dir, file, heading, &mut doc);
    }
    if !doc.is_empty() {
        doc = write_detail_section(doc);
    }
    let out_path = out.join(format!("{name}.md"));
    std::fs::write(out_path, doc).unwrap();
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

    let mapping = [
        ("as_changeset", "diesel_derives__tests__as_changeset_1.snap"),
        (
            "as_expression",
            "diesel_derives__tests__as_expression_1.snap",
        ),
        ("associations", "diesel_derives__tests__associations_1.snap"),
        ("auto_type", "diesel_derives__tests__auto_type_1.snap"),
        (
            "declare_sql_function",
            if has_sqlite {
                "diesel_derives__tests__declare_sql_function_1 (sqlite).snap"
            } else {
                "diesel_derives__tests__declare_sql_function_1.snap"
            },
        ),
        (
            "define_sql_function",
            if has_sqlite {
                "diesel_derives__tests__define_sql_function_1 (sqlite).snap"
            } else {
                "diesel_derives__tests__define_sql_function_1.snap"
            },
        ),
        ("from_sql_row", "diesel_derives__tests__from_sql_row_1.snap"),
        ("identifiable", "diesel_derives__tests__identifiable_1.snap"),
        ("insertable", "diesel_derives__tests__insertable_1.snap"),
        (
            "multiconnection",
            "diesel_derives__tests__multiconnection_1.snap",
        ),
        ("queryable", "diesel_derives__tests__queryable_1.snap"),
        (
            "queryable_by_name",
            "diesel_derives__tests__queryable_by_name_1.snap",
        ),
        ("query_id", "diesel_derives__tests__query_id_1.snap"),
        ("selectable", "diesel_derives__tests__selectable_1.snap"),
        ("table", "diesel_derives__tests__table_1.snap"),
        (
            "valid_grouping",
            "diesel_derives__tests__valid_grouping_1.snap",
        ),
    ];

    for (derive, file) in mapping {
        let (input, content) = read_snapshot(&snapshot_dir, file);
        let doc = normal_format(input, content);
        let out_path = out.join(format!("{derive}.md"));
        std::fs::write(out_path, doc).unwrap();
    }

    let mut sql_type = vec![];
    if has_sqlite {
        sql_type.push(("SQLite", "diesel_derives__tests__sql_type_1 (sqlite).snap"));
    }
    if has_postgres {
        sql_type.push((
            "PostgreSQL",
            "diesel_derives__tests__sql_type_1 (postgres).snap",
        ));
    }
    if has_mysql {
        sql_type.push(("MySQL", "diesel_derives__tests__sql_type_1 (mysql).snap"));
    }
    write_multiple(&snapshot_dir, &sql_type, "sql_type", &out);

    let mut has_query = vec![];
    if has_sqlite {
        has_query.push(("SQLite", "diesel_derives__tests__has_query_1 (sqlite).snap"));
    }
    if has_postgres {
        has_query.push((
            "PostgreSQL",
            "diesel_derives__tests__has_query_1 (postgres).snap",
        ));
    }
    if has_mysql {
        has_query.push(("MySQL", "diesel_derives__tests__has_query_1 (mysql).snap"));
    }
    write_multiple(&snapshot_dir, &has_query, "has_query", &out);
}

#[cfg(not(diesel_docsrs))]
fn main() {
    // just do nothing
}
