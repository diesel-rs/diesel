#![allow(clippy::expect_fun_call)] // My calls are so fun

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
            if fk.foreign_key_columns.len() > 1 {
                let first_pk = &fk.primary_key_columns[0];
                let all_same_pk = fk.primary_key_columns.iter().all(|pk| pk == first_pk);

                if all_same_pk {
                    tracing::debug!(?fk, "Extract first column from grouped foreign keys");
                    Some(ForeignKeyConstraint {
                        child_table: fk.child_table.clone(),
                        parent_table: fk.parent_table.clone(),
                        foreign_key_columns: vec![fk.foreign_key_columns[0].clone()],
                        foreign_key_columns_rust: vec![fk.foreign_key_columns_rust[0].clone()],
                        primary_key_columns: vec![fk.primary_key_columns[0].clone()],
                    })
                } else {
                    tracing::debug!(?fk, "Remove foreign key constraint because it's a compound foreign key");
                    None
                }
            } else if fk.foreign_key_columns.len() == 1 {
                Some(fk.clone())
            } else {
                None
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

/// Remove duplicate foreign keys for joinable! macro (only one relationship per table pair allowed)
pub fn remove_duplicated_foreign_keys(
    foreign_keys: &[ForeignKeyConstraint],
) -> Vec<ForeignKeyConstraint> {
    use std::collections::HashSet;

    let mut seen_table_pairs = HashSet::new();
    let mut result = Vec::new();

    for fk in foreign_keys {
        let ordered_tables = fk.ordered_tables();
        if seen_table_pairs.insert(ordered_tables) {
            result.push(fk.clone());
        } else {
            tracing::debug!(
                ?ordered_tables,
                "Remove foreign key constraint because another foreign key between these tables already exists"
            );
        }
    }

    result
}
