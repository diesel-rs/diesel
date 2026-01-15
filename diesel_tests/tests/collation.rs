use crate::schema::*;
use diesel::*;

#[diesel_test_helper::test]
#[cfg(feature = "sqlite")]
fn no_case_collation() {
    use crate::schema::users::dsl::*;
    use diesel::collation::NoCase;

    let connection = &mut connection();
    diesel::insert_into(users)
        .values(name.eq("Sean"))
        .execute(connection)
        .unwrap();

    let sean = users
        .filter(name.collate(NoCase).eq("sean"))
        .first::<User>(connection);
    assert_eq!(Ok(User::new(1, "Sean")), sean);

    let sean = users
        .filter(name.collate_nocase().eq("sean"))
        .first::<User>(connection);
    assert_eq!(Ok(User::new(1, "Sean")), sean);

    let sean = users
        .filter(name.collate_rtrim().eq("Sean   "))
        .first::<User>(connection);
    assert_eq!(Ok(User::new(1, "Sean")), sean);
}

#[diesel_test_helper::test]
#[cfg(feature = "sqlite")]
fn binary_collation() {
    use crate::schema::users::dsl::*;
    use diesel::collation::Binary;

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    // Test helper method
    let sean = users
        .filter(name.collate_binary().eq("Sean"))
        .load::<User>(connection);
    assert!(sean.is_ok());
    assert_eq!(1, sean.unwrap().len());

    // Test struct
    let sean = users
        .filter(name.collate(Binary).eq("Sean"))
        .load::<User>(connection);
    assert!(sean.is_ok());
    assert_eq!(1, sean.unwrap().len());

    // Case sensitivity check
    let sean = users
        .filter(name.collate_binary().eq("sean"))
        .load::<User>(connection);
    assert!(sean.is_ok());
    assert_eq!(0, sean.unwrap().len());
}

#[diesel_test_helper::test]
fn custom_collation() {
    use crate::schema::users::dsl::*;
    use diesel::collation::Custom;

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let target_collation = if cfg!(feature = "postgres") {
        Custom("\"C\"")
    } else if cfg!(feature = "mysql") {
        Custom("utf8mb4_bin")
    } else {
        Custom("BINARY")
    };

    let sean = users
        .filter(name.collate(target_collation).eq("Sean"))
        .load::<User>(connection);
    assert!(sean.is_ok());
    assert_eq!(1, sean.unwrap().len());

    let sean_lower = users
        .filter(name.collate(target_collation).eq("sean"))
        .load::<User>(connection);
    assert!(sean_lower.is_ok());

    // BINARY/C should be case sensitive, so "sean" != "Sean"
    assert_eq!(0, sean_lower.unwrap().len());
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn postgres_collations() {
    use crate::schema::users::dsl::*;
    use diesel::collation::{
        Default as DefaultCollation, PgCUtf8, PgUnicodeFast, Posix, UcsBasic, Unicode, C,
    };

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    // Verifying types and methods
    let _ = users
        .filter(name.collate(Posix).eq("Sean"))
        .load::<User>(connection)
        .unwrap();
    let _ = users
        .filter(name.collate_posix().eq("Sean"))
        .load::<User>(connection)
        .unwrap();

    let _ = users
        .filter(name.collate(C).eq("Sean"))
        .load::<User>(connection)
        .unwrap();
    let _ = users
        .filter(name.collate_c().eq("Sean"))
        .load::<User>(connection)
        .unwrap();

    let _ = users
        .filter(name.collate(Unicode).eq("Sean"))
        .load::<User>(connection)
        .unwrap();
    let _ = users
        .filter(name.collate_unicode().eq("Sean"))
        .load::<User>(connection)
        .unwrap();

    let _ = users
        .filter(name.collate(UcsBasic).eq("Sean"))
        .load::<User>(connection)
        .unwrap();
    let _ = users
        .filter(name.collate_ucs_basic().eq("Sean"))
        .load::<User>(connection)
        .unwrap();

    let _ = users
        .filter(name.collate(PgUnicodeFast).eq("Sean"))
        .load::<User>(connection)
        .unwrap();
    let _ = users
        .filter(name.collate_pg_unicode_fast().eq("Sean"))
        .load::<User>(connection)
        .unwrap();

    let _ = users
        .filter(name.collate(PgCUtf8).eq("Sean"))
        .load::<User>(connection)
        .unwrap();
    let _ = users
        .filter(name.collate_pg_c_utf8().eq("Sean"))
        .load::<User>(connection)
        .unwrap();

    let _ = users
        .filter(name.collate(DefaultCollation).eq("Sean"))
        .load::<User>(connection)
        .unwrap();
    let _ = users
        .filter(name.collate_default().eq("Sean"))
        .load::<User>(connection)
        .unwrap();
}
