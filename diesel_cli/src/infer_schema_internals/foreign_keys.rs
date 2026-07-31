#![allow(clippy::expect_fun_call)] // My calls are so fun

use std::collections::HashSet;

use super::data_structures::ForeignKeyConstraint;
use super::inference::get_primary_keys;
use super::table_data::TableName;
use crate::database::InferConnection;

/// Minimal filtering for allow_tables_to_appear_in_same_query! (keeps multi-column FKs and duplicates)
pub fn filter_foreign_keys_for_grouping(
    foreign_keys: &[ForeignKeyConstraint],
    safe_tables: &[TableName],
) -> Vec<ForeignKeyConstraint> {
    foreign_keys
        .iter()
        .filter(|fk| {
            if fk.parent_table == fk.child_table {
                tracing::debug!(?fk, "Remove foreign key constraint because it's self referential")
            }
            fk.parent_table != fk.child_table
        })
        .filter(|fk| {
            let parent_ok = safe_tables.contains(&fk.parent_table);
            let child_ok = safe_tables.contains(&fk.child_table);

            if !parent_ok || !child_ok {
                tracing::debug!(?fk, "Remove foreign key constraint because it references a table not in the outputted schema");
            }

            parent_ok && child_ok
        })
        .cloned()
        .collect()
}

pub fn remove_unsafe_foreign_keys_for_codegen(
    conn: &mut InferConnection,
    foreign_keys: &[ForeignKeyConstraint],
    safe_tables: &[TableName],
) -> Vec<ForeignKeyConstraint> {
    foreign_keys
        .iter()
        .filter(|fk| {
            if fk.parent_table == fk.child_table {
                tracing::debug!(?fk, "Remove foreign key constraint because it's self referential")
            }
            fk.parent_table != fk.child_table
        })
        .filter(|fk| {
            let parent_ok = safe_tables.contains(&fk.parent_table);
            let child_ok = safe_tables.contains(&fk.child_table);

            if !parent_ok || !child_ok {
                tracing::debug!(?fk, "Remove foreign key constraint because it references a table not in the outputted schema");
            }

            parent_ok && child_ok
        })
        .filter_map(|fk| {
            match fk.foreign_key_columns.len() {
                1 => Some(fk.clone()),
                x => {
                    if x > 1 {
                        tracing::debug!(?fk, "Remove foreign key constraint because it's a compound foreign key");
                    }
                    None
                }
            }
        })
        .filter(|fk| {
            let pk_columns = get_primary_keys(conn, &fk.parent_table).expect(&format!(
                "Error loading primary keys for `{}`",
                fk.parent_table
            ));
            let condition =
                pk_columns.len() == 1 && Some(&pk_columns[0]) == fk.primary_key_columns.first();
            if !condition {
                tracing::debug!(?fk, "Remove foreign key constraint because it references a table with several primary keys");
            }
            condition
        })
        .collect()
}

/// get a list of relations with several foreign key constraints between the same tables
pub fn duplicated_foreign_keys(
    foreign_keys: &[ForeignKeyConstraint],
) -> HashSet<(&TableName, &TableName)> {
    foreign_keys
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
        .collect()
}

/// Remove duplicate foreign keys for joinable! macro.
///
/// We only want to generate a `joinable!` entry if a single relation exists
pub fn remove_duplicated_foreign_keys(
    foreign_keys: &[ForeignKeyConstraint],
    duplicates: &HashSet<(&TableName, &TableName)>,
) -> Vec<ForeignKeyConstraint> {
    foreign_keys
        .iter()
        .filter(|fk| !duplicates.contains(&fk.ordered_tables()))
        .cloned()
        .collect()
}
