#[cfg(not(miri))]
use diesel::connection::InstrumentationEvent;
use diesel::connection::{Connection, SimpleConnection};
use diesel::result::Error;
use diesel::sql_types::{Bool, Integer, Text};
#[cfg(not(miri))]
use diesel::sqlite::ProgressDecision;
use diesel::sqlite::pragma::{
    ConnectionPragma, PerConnection, PerSchema, Pragma, PragmaLiteral, ReadPragma, SchemaPragma,
    WritePragma,
};
#[cfg(not(miri))]
use diesel::sqlite::{AuthorizerContext, AuthorizerDecision};
use diesel::sqlite::{AutoVacuumMode, SqliteConnection};
#[cfg(not(miri))]
use std::num::NonZeroU32;
#[cfg(not(miri))]
use std::sync::{Arc, Mutex};

fn connection() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

struct UserVersion(i32);

impl Pragma for UserVersion {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl ReadPragma for UserVersion {
    type SqlType = Integer;
}

impl WritePragma for UserVersion {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
}

struct AutoVacuum;

impl Pragma for AutoVacuum {
    const NAME: &'static str = "auto_vacuum";
    type Scope = PerSchema;
}

impl ReadPragma for AutoVacuum {
    type SqlType = Integer;
}

struct ForeignKeys(bool);

impl Pragma for ForeignKeys {
    const NAME: &'static str = "foreign_keys";
    type Scope = PerConnection;
}

impl ReadPragma for ForeignKeys {
    type SqlType = Bool;
}

impl WritePragma for ForeignKeys {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
}

struct IncrementalVacuum(u32);

impl Pragma for IncrementalVacuum {
    const NAME: &'static str = "incremental_vacuum";
    type Scope = PerSchema;
}

impl WritePragma for IncrementalVacuum {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
}

struct FreelistCount;

impl Pragma for FreelistCount {
    const NAME: &'static str = "freelist_count";
    type Scope = PerSchema;
}

impl ReadPragma for FreelistCount {
    type SqlType = diesel::sql_types::BigInt;
}

// Only used by tests that are compiled out under miri.
#[cfg(not(miri))]
struct DatabaseList;

#[cfg(not(miri))]
impl Pragma for DatabaseList {
    const NAME: &'static str = "database_list";
    type Scope = PerConnection;
}

#[cfg(not(miri))]
impl ReadPragma for DatabaseList {
    type SqlType = (Integer, Text, Text);
}

struct NoSuchPragma;

impl Pragma for NoSuchPragma {
    const NAME: &'static str = "no_such_pragma";
    type Scope = PerSchema;
}

impl ReadPragma for NoSuchPragma {
    type SqlType = Integer;
}

struct AssignmentInName;

impl Pragma for AssignmentInName {
    const NAME: &'static str = "user_version = 5";
    type Scope = PerSchema;
}

impl ReadPragma for AssignmentInName {
    type SqlType = Integer;
}

impl WritePragma for AssignmentInName {
    fn value(&self) -> PragmaLiteral<'_> {
        7.into()
    }
}

struct EmptyName;

impl Pragma for EmptyName {
    const NAME: &'static str = "";
    type Scope = PerSchema;
}

impl ReadPragma for EmptyName {
    type SqlType = Integer;
}

struct Synchronous;

impl Pragma for Synchronous {
    const NAME: &'static str = "synchronous";
    type Scope = PerSchema;
}

impl ReadPragma for Synchronous {
    type SqlType = Integer;
}

struct SynchronousKeyword(&'static str);

impl Pragma for SynchronousKeyword {
    const NAME: &'static str = "synchronous";
    type Scope = PerSchema;
}

impl WritePragma for SynchronousKeyword {
    fn value(&self) -> PragmaLiteral<'_> {
        PragmaLiteral::Keyword(self.0)
    }
}

struct TextValue<'a>(&'a str);

impl Pragma for TextValue<'_> {
    const NAME: &'static str = "test_text";
    type Scope = PerSchema;
}

impl WritePragma for TextValue<'_> {
    fn value(&self) -> PragmaLiteral<'_> {
        PragmaLiteral::Text(self.0)
    }
}

struct UserVersionAsPair;

impl Pragma for UserVersionAsPair {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl ReadPragma for UserVersionAsPair {
    type SqlType = (Integer, Text);
}

#[diesel_test_helper::test]
fn per_schema_get_reads_only_the_named_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();
    conn.batch_execute("PRAGMA aux.user_version = 5").unwrap();

    assert_eq!(5, UserVersion::get::<i32>(conn, Some("aux")).unwrap());
    assert_eq!(0, UserVersion::get::<i32>(conn, None).unwrap());
    assert_eq!(
        vec![5],
        UserVersion::get_all::<i32>(conn, Some("aux")).unwrap()
    );
    assert_eq!(vec![0], UserVersion::get_all::<i32>(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn caller_chooses_the_rust_type_of_a_row() {
    let conn = &mut connection();
    conn.batch_execute("PRAGMA auto_vacuum = INCREMENTAL")
        .unwrap();

    assert_eq!(2, AutoVacuum::get::<i32>(conn, None).unwrap());
    assert_eq!(
        AutoVacuumMode::Incremental,
        AutoVacuum::get::<AutoVacuumMode>(conn, None).unwrap()
    );
}

#[diesel_test_helper::test]
fn per_connection_get_reads_the_connection_setting() {
    let conn = &mut connection();

    conn.batch_execute("PRAGMA foreign_keys = ON").unwrap();
    assert!(ForeignKeys::get::<bool>(conn).unwrap());

    conn.batch_execute("PRAGMA foreign_keys = OFF").unwrap();
    assert!(!ForeignKeys::get::<bool>(conn).unwrap());
}

// Reads text allocated by the native sqlite library, whose provenance miri
// cannot track under `-Zmiri-native-lib`.
#[cfg(not(miri))]
#[diesel_test_helper::test]
fn connection_pragma_get_all_lists_every_attached_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();

    let names = DatabaseList::get_all::<(i32, String, String)>(conn)
        .unwrap()
        .into_iter()
        .map(|(_, name, _)| name)
        .collect::<Vec<_>>();

    assert_eq!(["main", "aux"], names.as_slice());
}

#[diesel_test_helper::test]
fn reading_an_unknown_pragma_is_not_found() {
    let conn = &mut connection();

    assert!(matches!(
        NoSuchPragma::get::<i32>(conn, None),
        Err(Error::NotFound)
    ));
}

#[diesel_test_helper::test]
fn name_outside_identifier_characters_is_rejected_before_execution() {
    let conn = &mut connection();

    assert!(matches!(
        AssignmentInName::get::<i32>(conn, None),
        Err(Error::QueryBuilderError(_))
    ));
    assert!(matches!(
        AssignmentInName.set(conn, None),
        Err(Error::QueryBuilderError(_))
    ));
    assert_eq!(0, UserVersion::get::<i32>(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn empty_name_is_rejected_before_execution() {
    let conn = &mut connection();

    assert!(matches!(
        EmptyName::get::<i32>(conn, None),
        Err(Error::QueryBuilderError(_))
    ));
}

#[diesel_test_helper::test]
fn keyword_outside_identifier_characters_is_rejected_before_execution() {
    let conn = &mut connection();
    let before = Synchronous::get::<i32>(conn, None).unwrap();

    let result = SynchronousKeyword("FULL; PRAGMA main.user_version = 7").set(conn, None);

    assert!(matches!(result, Err(Error::QueryBuilderError(_))));
    assert_eq!(before, Synchronous::get::<i32>(conn, None).unwrap());
    assert_eq!(0, UserVersion::get::<i32>(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn empty_keyword_is_rejected_before_execution() {
    let conn = &mut connection();

    assert!(matches!(
        SynchronousKeyword("").set(conn, None),
        Err(Error::QueryBuilderError(_))
    ));
}

#[cfg(not(miri))] // ffi string access
#[diesel_test_helper::test]
fn reading_all_rows_from_an_unknown_schema_is_a_database_error() {
    let conn = &mut connection();

    assert!(matches!(
        UserVersion::get_all::<i32>(conn, Some("missing")),
        Err(Error::DatabaseError(..))
    ));
}

#[diesel_test_helper::test]
fn row_not_matching_the_declared_type_is_a_deserialization_error() {
    let conn = &mut connection();

    assert!(matches!(
        UserVersionAsPair::get::<(i32, String)>(conn, None),
        Err(Error::DeserializationError(_))
    ));
    assert!(matches!(
        UserVersionAsPair::get_all::<(i32, String)>(conn, None),
        Err(Error::DeserializationError(_))
    ));
}

// Registers a progress callback that the native library invokes, which is not
// supported under miri with `-Zmiri-native-lib`.
#[cfg(not(miri))] // ffi call
#[diesel_test_helper::test]
fn interrupted_read_returns_the_interruption() {
    let conn = &mut connection();
    conn.on_progress(NonZeroU32::MIN, || ProgressDecision::Interrupt);

    assert!(matches!(
        UserVersion::get::<i32>(conn, None),
        Err(Error::DatabaseError(..))
    ));
    assert!(matches!(
        DatabaseList::get_all::<(i32, String, String)>(conn),
        Err(Error::DatabaseError(..))
    ));
}

#[diesel_test_helper::test]
fn per_schema_set_targets_only_the_named_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();

    UserVersion(5).set(conn, Some("aux")).unwrap();

    assert_eq!(5, UserVersion::get::<i32>(conn, Some("aux")).unwrap());
    assert_eq!(0, UserVersion::get::<i32>(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn integer_literal_round_trips_at_the_signed_boundary() {
    let conn = &mut connection();

    for value in [-1, i32::MIN, i32::MAX] {
        UserVersion(value).set(conn, None).unwrap();
        assert_eq!(value, UserVersion::get::<i32>(conn, None).unwrap());
    }
}

#[diesel_test_helper::test]
fn per_connection_bool_pragma_round_trips() {
    let conn = &mut connection();

    ForeignKeys(true).set(conn).unwrap();
    assert!(ForeignKeys::get::<bool>(conn).unwrap());

    ForeignKeys(false).set(conn).unwrap();
    assert!(!ForeignKeys::get::<bool>(conn).unwrap());
}

#[diesel_test_helper::test]
fn assignment_runs_to_completion() {
    let conn = &mut connection();
    conn.batch_execute(
        "PRAGMA auto_vacuum = INCREMENTAL;
         CREATE TABLE items (payload BLOB);
         INSERT INTO items VALUES (zeroblob(1000000));
         DELETE FROM items;",
    )
    .unwrap();
    assert!(FreelistCount::get::<i64>(conn, None).unwrap() > 1);

    // `incremental_vacuum` frees one page per step, so a single step would leave the rest.
    IncrementalVacuum(0).set(conn, None).unwrap();

    assert_eq!(0, FreelistCount::get::<i64>(conn, None).unwrap());
}

#[cfg(not(miri))] // ffi string access
#[diesel_test_helper::test]
fn setting_on_an_unknown_schema_is_a_database_error() {
    let conn = &mut connection();

    assert!(matches!(
        UserVersion(1).set(conn, Some("missing")),
        Err(Error::DatabaseError(..))
    ));
}

// Registers an authorizer callback that the native library invokes, which is
// not supported under miri with `-Zmiri-native-lib`.
#[cfg(not(miri))] // ffi call
#[diesel_test_helper::test]
fn borrowed_text_reaches_sqlite_as_one_uncached_value() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "a\"ux").unwrap();
    let arguments = Arc::new(Mutex::new(Vec::new()));
    let observed = arguments.clone();
    conn.on_authorize(move |context| {
        if let AuthorizerContext::Pragma(pragma) = context
            && pragma.name == Some("test_text")
        {
            observed.lock().unwrap().push((
                pragma.database.unwrap().to_owned(),
                pragma.argument.unwrap().to_owned(),
            ));
        }
        AuthorizerDecision::Allow
    });
    conn.set_instrumentation(|event: InstrumentationEvent<'_>| {
        assert!(!matches!(event, InstrumentationEvent::CacheQuery { .. }));
    });

    let values = [
        "",
        "runtime text",
        "x'0000000000000000000000000000000000000000000000000000000000000000'",
        "'\"\\\n\ré雪",
        "first''last",
        "'; PRAGMA main.user_version = 7; --",
    ];
    let mut expected = Vec::new();
    for schema in ["main", "a\"ux"] {
        for value in values {
            let value = value.to_owned();
            for _ in 0..2 {
                TextValue(&value).set(conn, Some(schema)).unwrap();
                expected.push((schema.to_owned(), value.clone()));
            }
        }
    }

    assert_eq!(expected, *arguments.lock().unwrap());
    conn.set_instrumentation(|_: InstrumentationEvent<'_>| {});
    assert_eq!(0, UserVersion::get::<i32>(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn text_with_nul_is_rejected_without_executing_a_prefix() {
    let conn = &mut connection();
    let value = String::from("text\0'; PRAGMA main.user_version = 7; --");

    assert!(matches!(
        TextValue(&value).set(conn, None),
        Err(Error::InvalidCString(_))
    ));
    assert_eq!(0, UserVersion::get::<i32>(conn, None).unwrap());
}
