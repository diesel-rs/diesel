use std::io::Write;

#[track_caller]
fn expand_with(
    input: proc_macro2::TokenStream,
    function: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
    name: &str,
) {
    let out = function(input);
    let out = out.to_string();

    let mut rustfmt = std::process::Command::new("rustfmt")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    {
        let mut stdin = rustfmt.stdin.take().unwrap();
        stdin.write_all(out.as_bytes()).unwrap();
    }
    let output = rustfmt.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8(output.stderr).unwrap()
    );
    let out = String::from_utf8(output.stdout).unwrap();

    insta::assert_snapshot!(name, out);
}

#[test]
fn as_changeset_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(input, crate::derive_as_changeset_inner, "as_changeset_1");
}

#[test]
fn as_expression_1() {
    let input = quote::quote! {
        #[diesel(sql_type = diesel::sql_type::Integer)]
        enum Foo {
            Bar,
            Baz
        }
    };
    expand_with(input, crate::derive_as_expression_inner, "as_expression_1");
}

#[test]
fn associations_1() {
    let input = quote::quote! {
        #[diesel(belongs_to(User))]
        struct Post {
            id: i32,
            title: String,
            user_id: i32,
        }
    };

    expand_with(input, crate::derive_associations_inner, "associations_1");
}

#[test]
fn diesel_numeric_ops_1() {
    let input = quote::quote! {
        struct NumericColumn;
    };

    expand_with(
        input,
        crate::derive_diesel_numeric_ops_inner,
        "diesel_numeric_ops_1",
    );
}

#[test]
fn from_sql_row_1() {
    let input = quote::quote! {
        enum Foo {
            Bar,
            Baz
        }
    };

    expand_with(input, crate::derive_from_sql_row_inner, "from_sql_row_1");
}

#[test]
fn identifiable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(input, crate::derive_identifiable_inner, "identifiable_1");
}

#[test]
fn insertable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
        }
    };

    expand_with(input, crate::derive_insertable_inner, "insertable_1");
}

#[test]
fn query_id_1() {
    let input = quote::quote! {
        struct Query;
    };

    expand_with(input, crate::derive_query_id_inner, "query_id_1");
}

#[test]
fn queryable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(input, crate::derive_queryable_inner, "queryable_1");
}

#[test]
fn queryable_by_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        input,
        crate::derive_queryable_by_name_inner,
        "queryable_by_name_1",
    );
}

#[test]
fn selectable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(input, crate::derive_selectable_inner, "selectable_1");
}

#[test]
fn sql_type_1() {
    let input = quote::quote! {
        #[diesel(postgres_type(oid = 42, array_oid = 142))]
        struct Integer;
    };

    expand_with(input, crate::derive_sql_type_inner, "sql_type_1");
}

#[test]
fn valid_grouping_1() {
    let input = quote::quote! {
        struct Query;
    };

    expand_with(
        input,
        crate::derive_valid_grouping_inner,
        "valid_grouping_1",
    );
}

#[test]

fn multiconnection_1() {
    let input = quote::quote! {
        enum DbConnection {
            Pg(PgConnection),
            Sqlite(diesel::SqliteConnection),
        }
    };

    expand_with(
        input,
        crate::derive_multiconnection_inner,
        "multiconnection_1",
    );
}

#[test]
fn table_1() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
        }
    };

    expand_with(input, crate::table_proc_inner, "table_1");
}

#[test]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
fn sql_function_1() {
    let input = quote::quote! {
        fn lower(input: Text) -> Text;
    };

    let name = if cfg!(feature = "sqlite") {
        "sql_function_1 (sqlite)"
    } else {
        "sql_function_1"
    };
    expand_with(input, crate::sql_function_proc_inner, name);
}

#[test]
fn define_sql_function_1() {
    let input = quote::quote! {
        fn lower(input: Text) -> Text;
    };

    let name = if cfg!(feature = "sqlite") {
        "define_sql_function_1 (sqlite)"
    } else {
        "define_sql_function_1"
    };
    expand_with(input, crate::define_sql_function_inner, name);
}

#[test]
fn auto_type_1() {
    let input = quote::quote! {
        fn foo() -> _ {
            users::table.select(users::id)
        }
    };
    expand_with(
        input,
        |input| crate::auto_type_inner(Default::default(), input),
        "auto_type_1",
    );
}

#[test]
fn declare_sql_function_1() {
    let input = quote::quote! {
        extern "SQL" {
            fn lower(input: Text) -> Text;
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "declare_sql_function_1 (sqlite)"
    } else {
        "declare_sql_function_1"
    };
    expand_with(
        input,
        |input| crate::declare_sql_function_inner(Default::default(), input),
        name,
    );
}

#[test]
fn diesel_for_each_tuple_1() {
    let input = quote::quote! {
        tuple_impls
    };

    expand_with(
        input,
        crate::__diesel_for_each_tuple_inner,
        "diesel_for_each_tuple_1",
    );
}

#[test]
fn diesel_public_if_1() {
    let input = quote::quote! {
        pub(crate) mod foo;
    };

    expand_with(
        input,
        |input| {
            crate::__diesel_public_if_inner(
                quote::quote! {
                feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
                        },
                input,
            )
        },
        "diesel_public_if_1",
    );
}
