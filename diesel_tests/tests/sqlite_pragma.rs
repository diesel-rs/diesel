use diesel::connection::Connection;
use diesel::result::Error;
use diesel::sql_types::{Bool, Integer, Text};
use diesel::sqlite::SqliteConnection;
use diesel::sqlite::pragma::{
    PerConnection, PerSchema, Pragma, PragmaLiteral, ReadPragma, ToPragmaLiteral, WritePragma,
};

fn connection() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

struct Synchronous;

#[derive(Clone, Copy)]
enum SynchronousMode {
    Extra,
}

impl ToPragmaLiteral for SynchronousMode {
    fn to_pragma_literal(&self) -> PragmaLiteral {
        match self {
            SynchronousMode::Extra => PragmaLiteral::Keyword("EXTRA"),
        }
    }
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
    type Value = SynchronousMode;
}

struct RawKeyword(&'static str);

impl ToPragmaLiteral for RawKeyword {
    fn to_pragma_literal(&self) -> PragmaLiteral {
        PragmaLiteral::Keyword(self.0)
    }
}

struct SynchronousKeyword;

impl Pragma for SynchronousKeyword {
    const NAME: &'static str = "synchronous";
    type Scope = PerSchema;
}

impl WritePragma for SynchronousKeyword {
    type Value = RawKeyword;
}

struct UserVersion;

impl Pragma for UserVersion {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl ReadPragma for UserVersion {
    type SqlType = Integer;
    type Row = i32;
}

impl WritePragma for UserVersion {
    type Value = i32;
}

struct ForeignKeys;

impl Pragma for ForeignKeys {
    const NAME: &'static str = "foreign_keys";
    type Scope = PerConnection;
}

impl ReadPragma for ForeignKeys {
    type SqlType = Bool;
    type Row = bool;
}

impl WritePragma for ForeignKeys {
    type Value = bool;
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

#[diesel_test_helper::test]
fn per_schema_keyword_pragma_targets_only_the_named_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();
    let main_before = conn.pragma::<Synchronous>(None).unwrap();

    conn.set_pragma::<Synchronous>(Some("aux"), &SynchronousMode::Extra)
        .unwrap();

    assert_eq!(3, conn.pragma::<Synchronous>(Some("aux")).unwrap());
    assert_eq!(main_before, conn.pragma::<Synchronous>(None).unwrap());
}

#[diesel_test_helper::test]
fn integer_literal_round_trips_at_the_signed_boundary() {
    let conn = &mut connection();

    for value in [-1, i32::MIN, i32::MAX] {
        conn.set_pragma::<UserVersion>(None, &value).unwrap();
        assert_eq!(value, conn.pragma::<UserVersion>(None).unwrap());
    }
}

#[diesel_test_helper::test]
fn per_connection_bool_pragma_round_trips() {
    let conn = &mut connection();

    conn.set_connection_pragma::<ForeignKeys>(&true).unwrap();
    assert!(conn.connection_pragma::<ForeignKeys>().unwrap());

    conn.set_connection_pragma::<ForeignKeys>(&false).unwrap();
    assert!(!conn.connection_pragma::<ForeignKeys>().unwrap());
}

#[diesel_test_helper::test]
fn connection_pragma_rows_lists_every_attached_database() {
    let conn = &mut connection();
    conn.attach_database(":memory:", "aux").unwrap();

    let names = conn
        .connection_pragma_rows::<DatabaseList>()
        .unwrap()
        .into_iter()
        .map(|(_, name, _)| name)
        .collect::<Vec<_>>();

    assert_eq!(["main", "aux"], names.as_slice());
}

#[diesel_test_helper::test]
fn keyword_outside_identifier_characters_is_rejected_before_execution() {
    let conn = &mut connection();
    let before = conn.pragma::<Synchronous>(None).unwrap();

    let result = conn
        .set_pragma::<SynchronousKeyword>(None, &RawKeyword("FULL; PRAGMA main.user_version = 7"));

    assert!(matches!(result, Err(Error::QueryBuilderError(_))));
    assert_eq!(before, conn.pragma::<Synchronous>(None).unwrap());
    assert_eq!(0, conn.pragma::<UserVersion>(None).unwrap());
}

#[diesel_test_helper::test]
fn reading_an_unknown_pragma_is_not_found() {
    let conn = &mut connection();

    assert!(matches!(
        conn.pragma::<NoSuchPragma>(None),
        Err(Error::NotFound)
    ));
}

#[diesel_test_helper::test]
fn name_outside_identifier_characters_is_rejected_before_execution() {
    let conn = &mut connection();

    let result = conn.pragma::<AssignmentInName>(None);

    assert!(matches!(result, Err(Error::QueryBuilderError(_))));
    assert_eq!(0, conn.pragma::<UserVersion>(None).unwrap());
}
