use diesel::connection::{Connection, InstrumentationEvent, SimpleConnection};
use diesel::query_dsl::RunQueryDsl;
use diesel::result::Error;
use diesel::sql_types::{Bool, Integer, Text};
use diesel::sqlite::pragma::{
    ConnectionPragma, PerConnection, PerSchema, Pragma, PragmaLiteral, ReadPragma, SchemaPragma,
    WritePragma, table_info,
};
use diesel::sqlite::{AuthorizerContext, AuthorizerDecision, ProgressDecision, SqliteConnection};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

fn connection() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

enum Synchronous {
    Extra,
}

impl Pragma for Synchronous {
    const NAME: &'static str = "synchronous";
    type Scope = PerSchema;
}

impl ReadPragma for Synchronous {
    type SqlType = Integer;
    type Row = i32;
}

impl WritePragma for Synchronous {
    fn value(&self) -> PragmaLiteral<'_> {
        match self {
            Synchronous::Extra => PragmaLiteral::Keyword("EXTRA"),
        }
    }
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

struct UserVersion(i32);

impl Pragma for UserVersion {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl ReadPragma for UserVersion {
    type SqlType = Integer;
    type Row = i32;
}

impl WritePragma for UserVersion {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
}

struct ForeignKeys(bool);

impl Pragma for ForeignKeys {
    const NAME: &'static str = "foreign_keys";
    type Scope = PerConnection;
}

impl ReadPragma for ForeignKeys {
    type SqlType = Bool;
    type Row = bool;
}

impl WritePragma for ForeignKeys {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
}

struct DatabaseList;

impl Pragma for DatabaseList {
    const NAME: &'static str = "database_list";
    type Scope = PerConnection;
}

impl ReadPragma for DatabaseList {
    type SqlType = (Integer, Text, Text);
    type Row = (i32, String, String);
}

struct NoSuchPragma;

impl Pragma for NoSuchPragma {
    const NAME: &'static str = "no_such_pragma";
    type Scope = PerSchema;
}

impl ReadPragma for NoSuchPragma {
    type SqlType = Integer;
    type Row = i32;
}

struct AssignmentInName;

impl Pragma for AssignmentInName {
    const NAME: &'static str = "user_version = 5";
    type Scope = PerSchema;
}

impl ReadPragma for AssignmentInName {
    type SqlType = Integer;
    type Row = i32;
}

struct EmptyName;

impl Pragma for EmptyName {
    const NAME: &'static str = "";
    type Scope = PerSchema;
}

impl ReadPragma for EmptyName {
    type SqlType = Integer;
    type Row = i32;
}

struct UserVersionAsPair;

impl Pragma for UserVersionAsPair {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl ReadPragma for UserVersionAsPair {
    type SqlType = (Integer, Text);
    type Row = (i32, String);
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

#[diesel_test_helper::test]
fn per_schema_keyword_pragma_targets_only_the_named_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();
    let main_before = Synchronous::get(conn, None).unwrap();

    Synchronous::Extra.set(conn, Some("aux")).unwrap();

    assert_eq!(3, Synchronous::get(conn, Some("aux")).unwrap());
    assert_eq!(main_before, Synchronous::get(conn, None).unwrap());
}

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
    assert_eq!(0, UserVersion::get(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn text_with_nul_is_rejected_without_executing_a_prefix() {
    let conn = &mut connection();
    let value = String::from("text\0'; PRAGMA main.user_version = 7; --");

    assert!(matches!(
        TextValue(&value).set(conn, None),
        Err(Error::InvalidCString(_))
    ));
    assert_eq!(0, UserVersion::get(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn incremental_vacuum_assignment_reclaims_the_whole_freelist() {
    use diesel::connection::SimpleConnection;
    use diesel::sqlite::AutoVacuumMode;

    let conn = &mut connection();
    conn.set_auto_vacuum(None, AutoVacuumMode::Incremental)
        .unwrap();
    conn.batch_execute(
        "CREATE TABLE items (payload BLOB);
         INSERT INTO items VALUES (zeroblob(1000000));
         DELETE FROM items;",
    )
    .unwrap();
    assert!(conn.freelist_count(None).unwrap() > 1);

    IncrementalVacuum(0).set(conn, None).unwrap();

    assert_eq!(0, conn.freelist_count(None).unwrap());
}

#[diesel_test_helper::test]
fn integer_literal_round_trips_at_the_signed_boundary() {
    let conn = &mut connection();

    for value in [-1, i32::MIN, i32::MAX] {
        UserVersion(value).set(conn, None).unwrap();
        assert_eq!(value, UserVersion::get(conn, None).unwrap());
    }
}

#[diesel_test_helper::test]
fn per_connection_bool_pragma_round_trips() {
    let conn = &mut connection();

    ForeignKeys(true).set(conn).unwrap();
    assert!(ForeignKeys::get(conn).unwrap());

    ForeignKeys(false).set(conn).unwrap();
    assert!(!ForeignKeys::get(conn).unwrap());
}

#[diesel_test_helper::test]
fn per_schema_get_all_reads_the_named_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();

    UserVersion(5).set(conn, Some("aux")).unwrap();

    assert_eq!(vec![5], UserVersion::get_all(conn, Some("aux")).unwrap());
    assert_eq!(vec![0], UserVersion::get_all(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn connection_pragma_get_all_lists_every_attached_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();

    let names = DatabaseList::get_all(conn)
        .unwrap()
        .into_iter()
        .map(|(_, name, _)| name)
        .collect::<Vec<_>>();

    assert_eq!(["main", "aux"], names.as_slice());
}

#[diesel_test_helper::test]
fn keyword_outside_identifier_characters_is_rejected_before_execution() {
    let conn = &mut connection();
    let before = Synchronous::get(conn, None).unwrap();

    let result = SynchronousKeyword("FULL; PRAGMA main.user_version = 7").set(conn, None);

    assert!(matches!(result, Err(Error::QueryBuilderError(_))));
    assert_eq!(before, Synchronous::get(conn, None).unwrap());
    assert_eq!(0, UserVersion::get(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn reading_an_unknown_pragma_is_not_found() {
    let conn = &mut connection();

    assert!(matches!(
        NoSuchPragma::get(conn, None),
        Err(Error::NotFound)
    ));
}

#[diesel_test_helper::test]
fn name_outside_identifier_characters_is_rejected_before_execution() {
    let conn = &mut connection();

    let result = AssignmentInName::get(conn, None);

    assert!(matches!(result, Err(Error::QueryBuilderError(_))));
    assert_eq!(0, UserVersion::get(conn, None).unwrap());
}

#[diesel_test_helper::test]
fn empty_name_or_keyword_is_rejected_before_execution() {
    let conn = &mut connection();

    assert!(matches!(
        EmptyName::get(conn, None),
        Err(Error::QueryBuilderError(_))
    ));
    assert!(matches!(
        SynchronousKeyword("").set(conn, None),
        Err(Error::QueryBuilderError(_))
    ));
}

#[diesel_test_helper::test]
fn reading_all_rows_from_an_unknown_schema_is_a_database_error() {
    let conn = &mut connection();

    assert!(matches!(
        UserVersion::get_all(conn, Some("missing")),
        Err(Error::DatabaseError(..))
    ));
}

#[diesel_test_helper::test]
fn row_not_matching_the_declared_type_is_a_deserialization_error() {
    let conn = &mut connection();

    assert!(matches!(
        UserVersionAsPair::get(conn, None),
        Err(Error::DeserializationError(_))
    ));
    assert!(matches!(
        UserVersionAsPair::get_all(conn, None),
        Err(Error::DeserializationError(_))
    ));
}

#[diesel_test_helper::test]
fn interrupted_read_returns_the_interruption() {
    let conn = &mut connection();
    conn.on_progress(NonZeroU32::MIN, || ProgressDecision::Interrupt);

    assert!(matches!(
        UserVersion::get(conn, None),
        Err(Error::DatabaseError(..))
    ));
    assert!(matches!(
        DatabaseList::get_all(conn),
        Err(Error::DatabaseError(..))
    ));
}

#[diesel_test_helper::test]
fn table_info_preserves_schema_resolution() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "a`ux").unwrap();
    conn.batch_execute(
        "CREATE TABLE shared (main_column INTEGER);
         CREATE TEMP TABLE shared (temp_column TEXT);
         CREATE TABLE `a``ux`.shared (attached_column INTEGER)",
    )
    .unwrap();

    type Column = (i32, String, String, bool, Option<String>, i32);
    for (schema, expected) in [
        (None, "temp_column"),
        (Some("main"), "main_column"),
        (Some("a`ux"), "attached_column"),
    ] {
        let columns = table_info("shared", schema).load::<Column>(conn).unwrap();
        assert_eq!(
            columns
                .into_iter()
                .map(|column| column.1)
                .collect::<Vec<_>>(),
            [expected],
        );
    }
}
