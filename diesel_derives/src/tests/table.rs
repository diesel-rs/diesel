use super::FunctionMacro;
use super::expand_with;

/// Tests that a basic table macro expands correctly.
///
/// The snapshot name varies based on the `postgres` feature because the generated
/// code includes additional PostgreSQL-specific trait implementations (e.g., `Only`,
/// `Tablesample`) when that feature is enabled. This requires separate snapshot files
/// to verify the output is correct for each configuration.
#[test]
pub(crate) fn table_1() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
        }
    };
    // Snapshot name varies by feature because postgres adds extra impls
    let name = if cfg!(feature = "postgres") {
        "table_1 (postgres)"
    } else {
        "table_1"
    };

    expand_with(
        &crate::table_proc_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(table)),
        name,
    );
}

/// Tests that a table with a feature-gated column expands correctly.
///
/// This verifies that columns with `#[cfg(...)]` attributes have their cfg guards
/// properly propagated to all generated trait implementations.
///
/// The snapshot name varies based on the `postgres` feature because the generated
/// code includes additional PostgreSQL-specific trait implementations when that
/// feature is enabled.
#[test]
pub(crate) fn table_with_column_feature_gate() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
            #[cfg(feature = "chrono")]
            created_at -> Timestamp,
        }
    };
    // Snapshot name varies by feature because postgres adds extra impls
    let name = if cfg!(feature = "postgres") {
        "table_with_column_feature_gate (postgres)"
    } else {
        "table_with_column_feature_gate"
    };

    expand_with(
        &crate::table_proc_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(table)),
        name,
    );
}

/// Tests that a table with multiple feature-gated columns (with different features)
/// expands correctly.
///
/// This verifies that when columns have different cfg conditions, the macro generates
/// 2^n combinatorial variants of aggregate types (`all_columns`, `SqlType`, `AllColumns`)
/// where n is the number of distinct cfg groups.
///
/// The snapshot name varies based on the `postgres` feature because the generated
/// code includes additional PostgreSQL-specific trait implementations when that
/// feature is enabled.
#[test]
pub(crate) fn table_with_multiple_feature_gated_columns() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
            #[cfg(feature = "chrono")]
            created_at -> Timestamp,
            #[cfg(feature = "uuid")]
            user_uuid -> Uuid,
            #[cfg(feature = "chrono")]
            updated_at -> Timestamp,
        }
    };
    // Snapshot name varies by feature because postgres adds extra impls
    let name = if cfg!(feature = "postgres") {
        "table_with_multiple_feature_gated_columns (postgres)"
    } else {
        "table_with_multiple_feature_gated_columns"
    };

    expand_with(
        &crate::table_proc_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(table)),
        name,
    );
}

/// This test verifies that feature-gated columns produce the correct aggregate types
/// depending on which features are enabled. The test checks:
/// - `all_columns` const type
/// - `SqlType` type alias
/// - `Table::AllColumns` associated type
///
/// For a table with one feature-gated column (`#[cfg(feature = "chrono")] created_at`):
/// - Without chrono: types should be `(id, name)` / `(Integer, Text)`
/// - With chrono: types should be `(id, name, created_at)` / `(Integer, Text, Timestamp)`
#[test]
pub(crate) fn table_with_column_feature_gate_type_check() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
            #[cfg(feature = "test_chrono")]
            created_at -> Timestamp,
        }
    };

    // Generate the table code
    let generated = crate::table_proc_inner(input);
    let generated_str = generated.to_string();

    // Verify that appropriate cfg guards are present for all_columns
    assert!(
        generated_str.contains("# [cfg (all (not (feature = \"test_chrono\")))]"),
        "Should have cfg guard for non-chrono all_columns variant"
    );
    assert!(
        generated_str.contains("# [cfg (all (feature = \"test_chrono\"))]"),
        "Should have cfg guard for chrono all_columns variant"
    );

    // Verify correct column tuples in all_columns (note: trailing commas in generated code)
    // Non-gated variant should have (id, name,)
    assert!(
        generated_str.contains("pub const all_columns : (id , name ,) = (id , name ,)"),
        "Non-gated all_columns should be (id, name)"
    );
    // Gated variant should have (id, name, created_at,)
    assert!(
        generated_str.contains("pub const all_columns : (id , name , created_at ,) = (id , name , created_at ,)"),
        "Gated all_columns should include created_at"
    );

    // Verify correct SqlType variants
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text ,) ;"),
        "Non-gated SqlType should be (Integer, Text)"
    );
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text , Timestamp ,) ;"),
        "Gated SqlType should include Timestamp"
    );

    // Verify Table impl has correct AllColumns type
    assert!(
        generated_str.contains("type AllColumns = (id , name ,) ;"),
        "Non-gated Table::AllColumns should be (id, name)"
    );
    assert!(
        generated_str.contains("type AllColumns = (id , name , created_at ,) ;"),
        "Gated Table::AllColumns should include created_at"
    );
}

/// This test verifies that multiple feature-gated columns with different features
/// produce the correct combinatorial variants (2^n for n distinct feature groups).
///
/// For a table with two feature groups (`chrono` and `uuid`):
/// - Neither enabled: `(id, name)`
/// - Only chrono: `(id, name, created_at, updated_at)`
/// - Only uuid: `(id, name, user_uuid)`
/// - Both enabled: `(id, name, created_at, updated_at, user_uuid)`
#[test]
pub(crate) fn table_with_multiple_feature_gated_columns_type_check() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
            #[cfg(feature = "test_chrono")]
            created_at -> Timestamp,
            #[cfg(feature = "test_uuid")]
            user_uuid -> Uuid,
            #[cfg(feature = "test_chrono")]
            updated_at -> Timestamp,
        }
    };

    let generated = crate::table_proc_inner(input);
    let generated_str = generated.to_string();

    // Verify all 4 cfg combinations are present
    assert!(
        generated_str.contains("# [cfg (all (not (feature = \"test_chrono\") , not (feature = \"test_uuid\")))]"),
        "Should have cfg guard for neither feature enabled"
    );
    assert!(
        generated_str.contains("# [cfg (all (feature = \"test_chrono\" , not (feature = \"test_uuid\")))]"),
        "Should have cfg guard for only chrono enabled"
    );
    assert!(
        generated_str.contains("# [cfg (all (not (feature = \"test_chrono\") , feature = \"test_uuid\"))]"),
        "Should have cfg guard for only uuid enabled"
    );
    assert!(
        generated_str.contains("# [cfg (all (feature = \"test_chrono\" , feature = \"test_uuid\"))]"),
        "Should have cfg guard for both features enabled"
    );

    // Verify all_columns variants (note: trailing commas in generated code)
    // Neither feature: (id, name,)
    assert!(
        generated_str.contains("pub const all_columns : (id , name ,) = (id , name ,)"),
        "Neither feature: all_columns should be (id, name)"
    );
    // Only chrono: (id, name, created_at, updated_at,)
    assert!(
        generated_str.contains("pub const all_columns : (id , name , created_at , updated_at ,)"),
        "Only chrono: all_columns should include chrono columns"
    );
    // Only uuid: (id, name, user_uuid,)
    assert!(
        generated_str.contains("pub const all_columns : (id , name , user_uuid ,)"),
        "Only uuid: all_columns should include uuid column"
    );
    // Both features: (id, name, created_at, updated_at, user_uuid,)
    assert!(
        generated_str.contains("pub const all_columns : (id , name , created_at , updated_at , user_uuid ,)"),
        "Both features: all_columns should include all columns"
    );

    // Verify SqlType variants
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text ,) ;"),
        "Neither feature: SqlType should be (Integer, Text)"
    );
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text , Timestamp , Timestamp ,) ;"),
        "Only chrono: SqlType should have two Timestamps"
    );
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text , Uuid ,) ;"),
        "Only uuid: SqlType should include Uuid"
    );
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text , Timestamp , Timestamp , Uuid ,) ;"),
        "Both features: SqlType should include all types"
    );

    // Verify Table::AllColumns variants
    assert!(
        generated_str.contains("type AllColumns = (id , name ,) ;"),
        "Neither feature: AllColumns should be (id, name)"
    );
    assert!(
        generated_str.contains("type AllColumns = (id , name , created_at , updated_at ,) ;"),
        "Only chrono: AllColumns should include chrono columns"
    );
    assert!(
        generated_str.contains("type AllColumns = (id , name , user_uuid ,) ;"),
        "Only uuid: AllColumns should include uuid column"
    );
    assert!(
        generated_str.contains("type AllColumns = (id , name , created_at , updated_at , user_uuid ,) ;"),
        "Both features: AllColumns should include all columns"
    );
}

/// This test verifies that ValidGrouping for star has correct combinatorial variants.
#[test]
pub(crate) fn table_with_feature_gate_valid_grouping_star_check() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
            #[cfg(feature = "test_feature")]
            extra -> Text,
        }
    };

    let generated = crate::table_proc_inner(input);
    let generated_str = generated.to_string();

    // Verify ValidGrouping<__GB> for star has both variants (note: trailing commas)
    // Non-gated: (id, name,)
    assert!(
        generated_str.contains("(id , name ,) : diesel :: expression :: ValidGrouping < __GB >"),
        "Non-gated ValidGrouping for star should use (id, name)"
    );
    // Gated: (id, name, extra,)
    assert!(
        generated_str.contains("(id , name , extra ,) : diesel :: expression :: ValidGrouping < __GB >"),
        "Gated ValidGrouping for star should use (id, name, extra)"
    );
}
