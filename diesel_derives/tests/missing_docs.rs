//! Compile test asserting that `diesel::table!` and `diesel::view!` output is
//! clean under `missing_docs` at the strictest lint level.
#![forbid(missing_docs)]

extern crate diesel;

// table, no docs: module and both column structs are hidden.
diesel::table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

// table, fully documented: nothing is hidden, the caller's docs stay visible.
diesel::table! {
    /// The posts a user has written.
    posts {
        /// Primary key of the post.
        id -> Integer,
        /// Free-form title text.
        title -> Text,
    }
}

// table doc present, column docs absent: the module stays visible while the
// undocumented column structs are hidden.
diesel::table! {
    /// Comments left on posts.
    comments {
        id -> Integer,
        body -> Text,
    }
}

// table doc absent, column docs present: the module is hidden, which propagates
// to its columns, so documented columns compile under an undocumented table.
diesel::table! {
    tags {
        /// The tag id.
        id -> Integer,
        /// The tag label.
        label -> Text,
    }
}

// view, no docs: the shared generator hides the module and column structs.
diesel::view! {
    active_users {
        id -> Integer,
        name -> Text,
    }
}

// view, fully documented: the caller's docs stay visible.
diesel::view! {
    /// Users seen in the last 30 days.
    recent_users {
        /// The user id.
        id -> Integer,
        /// The user's display name.
        name -> Text,
    }
}

#[test]
fn schema_macros_are_missing_docs_clean() {
    // The contract under test is that the schema declarations above compile
    // under `forbid(missing_docs)`.
    let _ = users::table;
    let _ = posts::table;
    let _ = comments::table;
    let _ = tags::table;
    let _ = active_users::table;
    let _ = recent_users::table;
}
