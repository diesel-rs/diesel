use schema::*;
use diesel::*;

#[test]
fn ci_text_exists_and_coerces_from_text() {
    use schema::citext_table::dsl::*;

    let conn = connection();
    conn.execute(
        "INSERT INTO citext_table (citext_field) VALUES ('foo'::citext), ('bar'::citext)",
    ).unwrap();
    let data = citext_table
        .filter(citext_field.eq("foo"))
        .select(citext_field)
        .load::<String>(&conn);
    let expected = vec!["foo".into()];

    assert_eq!(Ok(expected), data);
}
