#![allow(clippy::expect_fun_call)] // My calls are so fun

use super::data_structures::ForeignKeyConstraint;
use super::inference::get_primary_keys;
use super::table_data::TableName;
use crate::database::InferConnection;

pub fn remove_unsafe_foreign_keys_for_codegen(
    conn: &mut InferConnection,
    foreign_keys: &[ForeignKeyConstraint],
    safe_tables: &[TableName],
) -> Vec<ForeignKeyConstraint> {
    let duplicates = foreign_keys
        .iter()
        .map(ForeignKeyConstraint::ordered_tables)
        .filter(|tables| {
            let dup_count = foreign_keys
                .iter()
                .filter(|fk| tables == &fk.ordered_tables())
                .count();
            if dup_count > 1 {
                tracing::debug!(
                    ?tables,
                    "Remove foreign key constraint because it's not unique"
                );
            }
            dup_count > 1
        })
        .collect::<Vec<_>>();

    foreign_keys
        .iter()
        .filter(|fk| {
            if fk.parent_table == fk.child_table {
                tracing::debug!(?fk, "Remove foreign key constraint because it's self referential")
            }
            fk.parent_table != fk.child_table
        })
        .filter(|fk|{
            if !safe_tables.contains(&fk.parent_table) {
                tracing::debug!(?fk, "Remove foreign key constraint because it references a table not in the outputted schema");
            }
            safe_tables.contains(&fk.parent_table)
        })
        .filter(|fk| {
            if !safe_tables.contains(&fk.child_table) {
                tracing::debug!(?fk, "Remove foreign key constraint because it references a table not in the outputted schema");
            }
            safe_tables.contains(&fk.child_table)
        })
        .filter(|fk| {
            if fk.foreign_key_columns.len() != 1 {
                tracing::debug!(?fk, "Remove foreign key constraint because it contains several foreign keys");
            }
            fk.foreign_key_columns.len() == 1
        })
        .filter(|fk| {
            let pk_columns = get_primary_keys(conn, &fk.parent_table).expect(&format!(
                "Error loading primary keys for `{}`",
                fk.parent_table
            ));
            let condition = pk_columns.len() == 1 && Some(&pk_columns[0]) == fk.primary_key_columns.first();
            if !condition {
                tracing::debug!(?fk, "Remove foreign key constraint because it references a table with several primary keys");
            }
            condition
        })
        .filter(|fk| !duplicates.contains(&fk.ordered_tables()))
        .cloned()
        .collect()
}
