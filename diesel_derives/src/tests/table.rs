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
/// - `all_columns` const (single declaration with cfg-on-tuple-expression-field)
/// - `SqlType` type alias (2^n cfg-gated variants)
/// - `AllColumns` type alias (2^n cfg-gated variants)
/// - `Table::AllColumns` associated type (single declaration referencing the `AllColumns` alias)
///
/// For a table with one feature-gated column (`#[cfg(feature = "test_chrono")] created_at`):
/// - Without `test_chrono`: aliases resolve to `(id, name)` / `(Integer, Text)`
/// - With `test_chrono`: aliases resolve to `(id, name, created_at)` / `(Integer, Text, Timestamp)`
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

    // Both cfg guards (enabled/disabled) must appear so the 2^n tuple-type
    // aliases are selected correctly per build configuration.
    assert!(
        generated_str.contains("# [cfg (all (not (feature = \"test_chrono\")))]"),
        "Should have cfg guard for non-chrono variant"
    );
    assert!(
        generated_str.contains("# [cfg (all (feature = \"test_chrono\"))]"),
        "Should have cfg guard for chrono variant"
    );

    // `all_columns` is a single const using the `AllColumns` type alias and
    // cfg-on-tuple-expression-field for the gated column.
    assert!(
        generated_str.contains(
            "pub const all_columns : AllColumns = (id , name , # [cfg (feature = \"test_chrono\")] created_at ,)"
        ),
        "all_columns should be a single const referencing AllColumns alias with cfg on the gated tuple field"
    );

    // SqlType type alias retains 2^n variants (one per cfg combination)
    // because Rust does not allow #[cfg] on tuple type fields.
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text ,) ;"),
        "Non-gated SqlType should be (Integer, Text)"
    );
    assert!(
        generated_str.contains("pub type SqlType = (Integer , Text , Timestamp ,) ;"),
        "Gated SqlType should include Timestamp"
    );

    // AllColumns type alias mirrors SqlType: 2^n cfg-gated tuple type variants.
    assert!(
        generated_str.contains("pub type AllColumns = (id , name ,) ;"),
        "Non-gated AllColumns alias should be (id, name)"
    );
    assert!(
        generated_str.contains("pub type AllColumns = (id , name , created_at ,) ;"),
        "Gated AllColumns alias should include created_at"
    );

    // `impl Table for table` is a single block that references the aliases.
    assert!(
        generated_str.contains("type AllColumns = AllColumns ;"),
        "Table::AllColumns associated type should reference the AllColumns alias"
    );
}

/// This test verifies that multiple feature-gated columns with different features
/// produce the correct combinatorial variants (2^n for n distinct feature groups)
/// in the `SqlType` and `AllColumns` type aliases, while the `all_columns` const
/// and the `Table` impl collapse to single declarations.
///
/// For a table with two feature groups (`test_chrono` and `test_uuid`):
/// - Neither enabled: aliases resolve to `(id, name)` / `(Integer, Text)`
/// - Only chrono: `(id, name, created_at, updated_at)` / `(Integer, Text, Timestamp, Timestamp)`
/// - Only uuid: `(id, name, user_uuid)` / `(Integer, Text, Uuid)`
/// - Both enabled: `(id, name, created_at, updated_at, user_uuid)` / `(Integer, Text, Timestamp, Timestamp, Uuid)`
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

    // All 4 cfg combinations must appear as cfg guards on the 2^n tuple-type aliases.
    assert!(
        generated_str.contains(
            "# [cfg (all (not (feature = \"test_chrono\") , not (feature = \"test_uuid\")))]"
        ),
        "Should have cfg guard for neither feature enabled"
    );
    assert!(
        generated_str
            .contains("# [cfg (all (feature = \"test_chrono\" , not (feature = \"test_uuid\")))]"),
        "Should have cfg guard for only chrono enabled"
    );
    assert!(
        generated_str
            .contains("# [cfg (all (not (feature = \"test_chrono\") , feature = \"test_uuid\"))]"),
        "Should have cfg guard for only uuid enabled"
    );
    assert!(
        generated_str
            .contains("# [cfg (all (feature = \"test_chrono\" , feature = \"test_uuid\"))]"),
        "Should have cfg guard for both features enabled"
    );

    // Single `all_columns` const uses cfg-on-tuple-expression-field for every gated column.
    assert!(
        generated_str.contains(
            "pub const all_columns : AllColumns = (id , name , \
             # [cfg (feature = \"test_chrono\")] created_at , \
             # [cfg (feature = \"test_chrono\")] updated_at , \
             # [cfg (feature = \"test_uuid\")] user_uuid ,)"
        ),
        "all_columns should be a single const referencing AllColumns with cfg on each gated tuple field"
    );

    // SqlType type alias keeps 2^n variants.
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
        generated_str
            .contains("pub type SqlType = (Integer , Text , Timestamp , Timestamp , Uuid ,) ;"),
        "Both features: SqlType should include all types"
    );

    // AllColumns type alias also keeps 2^n variants.
    assert!(
        generated_str.contains("pub type AllColumns = (id , name ,) ;"),
        "Neither feature: AllColumns alias should be (id, name)"
    );
    assert!(
        generated_str.contains("pub type AllColumns = (id , name , created_at , updated_at ,) ;"),
        "Only chrono: AllColumns alias should include chrono columns"
    );
    assert!(
        generated_str.contains("pub type AllColumns = (id , name , user_uuid ,) ;"),
        "Only uuid: AllColumns alias should include uuid column"
    );
    assert!(
        generated_str.contains(
            "pub type AllColumns = (id , name , created_at , updated_at , user_uuid ,) ;"
        ),
        "Both features: AllColumns alias should include all columns"
    );

    // The Table impl's associated type is a single reference to the alias.
    assert!(
        generated_str.contains("type AllColumns = AllColumns ;"),
        "Table::AllColumns associated type should reference the AllColumns alias"
    );
}

/// Verifies that `impl ValidGrouping<__GB> for star` is emitted as a single impl
/// that references the module-level `AllColumns` type alias (which itself carries
/// the 2^n cfg-gated variants).
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

    // Single where-bound on the AllColumns alias, no inlined tuple.
    assert!(
        generated_str
            .contains("super :: AllColumns : diesel :: expression :: ValidGrouping < __GB >"),
        "ValidGrouping for star should reference super::AllColumns rather than an inlined column tuple"
    );
    assert!(
        generated_str.contains(
            "< super :: AllColumns as diesel :: expression :: ValidGrouping < __GB >> :: IsAggregate"
        ),
        "IsAggregate projection should be on super::AllColumns"
    );
}

/// Tests that complex cfg conditions are parsed and propagated correctly.
///
/// This verifies that cfg conditions beyond simple `#[cfg(feature = "x")]` work:
/// - `#[cfg(all(feature = "a", feature = "b"))]` - requires both features
/// - `#[cfg(any(feature = "a", feature = "b"))]` - requires either feature
/// - `#[cfg(not(feature = "a"))]` - requires feature to be disabled
/// - `#[cfg(all(feature = "a", not(feature = "b")))]` - complex combination
///
/// The macro should preserve these conditions exactly as specified on the column,
/// propagating them to all generated trait implementations.
#[test]
pub(crate) fn table_with_complex_feature_gates() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
            #[cfg(all(feature = "postgres", feature = "chrono"))]
            pg_created_at -> Timestamptz,
            #[cfg(any(feature = "sqlite", feature = "mysql"))]
            simple_created_at -> Timestamp,
            #[cfg(not(feature = "production"))]
            debug_info -> Text,
            #[cfg(all(feature = "advanced", not(feature = "lite")))]
            advanced_field -> Text,
        }
    };

    let generated = crate::table_proc_inner(input);
    let generated_str = generated.to_string();

    // Verify that the complex cfg conditions are preserved in column definitions
    // all(feature = "postgres", feature = "chrono")
    assert!(
        generated_str.contains("# [cfg (all (feature = \"postgres\" , feature = \"chrono\"))]"),
        "Should preserve all(feature, feature) cfg condition"
    );

    // any(feature = "sqlite", feature = "mysql")
    assert!(
        generated_str.contains("# [cfg (any (feature = \"sqlite\" , feature = \"mysql\"))]"),
        "Should preserve any(feature, feature) cfg condition"
    );

    // not(feature = "production")
    assert!(
        generated_str.contains("# [cfg (not (feature = \"production\"))]"),
        "Should preserve not(feature) cfg condition"
    );

    // all(feature = "advanced", not(feature = "lite"))
    assert!(
        generated_str.contains("# [cfg (all (feature = \"advanced\" , not (feature = \"lite\")))]"),
        "Should preserve all(feature, not(feature)) cfg condition"
    );

    // Verify the column structs are generated with their cfg attributes
    assert!(
        generated_str.contains("pub struct pg_created_at"),
        "pg_created_at column struct should be generated"
    );
    assert!(
        generated_str.contains("pub struct simple_created_at"),
        "simple_created_at column struct should be generated"
    );
    assert!(
        generated_str.contains("pub struct debug_info"),
        "debug_info column struct should be generated"
    );
    assert!(
        generated_str.contains("pub struct advanced_field"),
        "advanced_field column struct should be generated"
    );

    // Verify that Expression impl for each column has the cfg attribute
    // (checking that trait impls are properly gated)
    assert!(
        generated_str.contains("# [cfg (all (feature = \"postgres\" , feature = \"chrono\"))] impl diesel :: expression :: Expression for pg_created_at"),
        "Expression impl for pg_created_at should have cfg guard"
    );
    assert!(
        generated_str.contains("# [cfg (any (feature = \"sqlite\" , feature = \"mysql\"))] impl diesel :: expression :: Expression for simple_created_at"),
        "Expression impl for simple_created_at should have cfg guard"
    );
    assert!(
        generated_str.contains("# [cfg (not (feature = \"production\"))] impl diesel :: expression :: Expression for debug_info"),
        "Expression impl for debug_info should have cfg guard"
    );
    assert!(
        generated_str.contains("# [cfg (all (feature = \"advanced\" , not (feature = \"lite\")))] impl diesel :: expression :: Expression for advanced_field"),
        "Expression impl for advanced_field should have cfg guard"
    );

    // `all_columns` is now a single const using cfg-on-tuple-expression-field for
    // every gated column. The base case (no features) corresponds to the alias
    // resolving to `(id, name)` at the type level; the const itself stays singular.
    assert!(
        generated_str.contains("pub const all_columns : AllColumns = (id , name ,"),
        "all_columns should be a single const referencing the AllColumns alias"
    );

    // The 2^n cfg guards on the SqlType / AllColumns aliases must include the
    // negated forms of every group's complex cfg condition.
    assert!(
        generated_str.contains("not (all (feature = \"postgres\" , feature = \"chrono\"))"),
        "Combinatorial cfg should include negated complex conditions"
    );
    assert!(
        generated_str.contains("not (any (feature = \"sqlite\" , feature = \"mysql\"))"),
        "Combinatorial cfg should include negated any() conditions"
    );
}

/// Tests that IsContainedInGroupBy impls between columns with complex cfg conditions
/// include cfg attributes from both columns.
#[test]
pub(crate) fn table_with_complex_feature_gates_cross_column_impls() {
    let input = quote::quote! {
        users {
            id -> Integer,
            #[cfg(all(feature = "a", feature = "b"))]
            col_ab -> Text,
            #[cfg(any(feature = "c", feature = "d"))]
            col_cd -> Text,
        }
    };

    let generated = crate::table_proc_inner(input);
    let generated_str = generated.to_string();

    // The IsContainedInGroupBy impl between col_ab and col_cd should have both cfg conditions
    // This ensures the impl only exists when both columns are available.
    // Note: The order of cfg attributes follows the order columns appear in the impl
    // (left column's cfg attrs come first, then right column's cfg attrs)
    assert!(
        generated_str.contains("# [cfg (any (feature = \"c\" , feature = \"d\"))] # [cfg (all (feature = \"a\" , feature = \"b\"))] impl diesel :: expression :: IsContainedInGroupBy < col_cd > for col_ab"),
        "IsContainedInGroupBy<col_cd> for col_ab should have cfg attrs from both columns"
    );
    assert!(
        generated_str.contains("# [cfg (any (feature = \"c\" , feature = \"d\"))] # [cfg (all (feature = \"a\" , feature = \"b\"))] impl diesel :: expression :: IsContainedInGroupBy < col_ab > for col_cd"),
        "IsContainedInGroupBy<col_ab> for col_cd should have cfg attrs from both columns"
    );
}
