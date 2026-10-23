use diesel::connection::{Connection, SimpleConnection};
use diesel::result::Error;
use diesel::sql_types::{Bool, Integer, Text};
#[cfg(not(miri))]
use diesel::sqlite::ProgressDecision;
use diesel::sqlite::pragma::{
    ConnectionPragma, PerConnection, PerSchema, Pragma, ReadPragma, SchemaPragma,
};
use diesel::sqlite::{AutoVacuumMode, SqliteConnection};
#[cfg(not(miri))]
use std::num::NonZeroU32;

fn connection() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

struct UserVersion;

impl Pragma for UserVersion {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl ReadPragma for UserVersion {
    type SqlType = Integer;
}

struct AutoVacuum;

impl Pragma for AutoVacuum {
    const NAME: &'static str = "auto_vacuum";
    type Scope = PerSchema;
}

impl ReadPragma for AutoVacuum {
    type SqlType = Integer;
}

struct ForeignKeys;

impl Pragma for ForeignKeys {
    const NAME: &'static str = "foreign_keys";
    type Scope = PerConnection;
}

impl ReadPragma for ForeignKeys {
    type SqlType = Bool;
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

struct EmptyName;

impl Pragma for EmptyName {
    const NAME: &'static str = "";
    type Scope = PerSchema;
}

impl ReadPragma for EmptyName {
    type SqlType = Integer;
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

    let result = AssignmentInName::get::<i32>(conn, None);

    assert!(matches!(result, Err(Error::QueryBuilderError(_))));
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
