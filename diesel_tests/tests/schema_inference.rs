#[cfg(feature = "sqlite")]
mod sqlite {
    use diesel::*;
    use schema::*;

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[table_name="infer_all_the_ints"]
    struct InferredInts {
        col1: i32,
        col2: i32,
        col3: i32,
        col4: i32,
        col5: i16,
        col6: i16,
        col7: i16,
        col8: i64,
        col9: i64,
        col10: i64,
        col11: i16,
        col12: i32,
        col13: i64,
    }

    #[test]
    fn integers_infer_to_semantically_correct_types() {
        let conn = connection();
        let inferred_ints = InferredInts {
            col1: 1,
            col2: 2,
            col3: 3,
            col4: 4,
            col5: 5,
            col6: 6,
            col7: 7,
            col8: 8,
            col9: 9,
            col10: 10,
            col11: 11,
            col12: 12,
            col13: 13,
        };
        insert(&inferred_ints).into(infer_all_the_ints::table)
            .execute(&conn).unwrap();

        assert_eq!(Ok(vec![inferred_ints]), infer_all_the_ints::table.load(&conn));
    }

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[table_name="infer_all_the_bools"]
    struct InferredBools {
        col1: bool,
        col2: bool,
        col3: bool,
        col4: bool,
    }

    #[test]
    fn bool_types_infer_to_bool() {
        let conn = connection();
        let inferred_bools = InferredBools {
            col1: true,
            col2: true,
            col3: false,
            col4: false,
        };
        insert(&inferred_bools).into(infer_all_the_bools::table)
            .execute(&conn).unwrap();

        assert_eq!(Ok(vec![inferred_bools]), infer_all_the_bools::table.load(&conn));
    }

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[table_name="infer_all_the_strings"]
    struct InferredStrings {
        col1: String,
        col2: String,
        col3: String,
        col4: String,
        col5: String,
        col6: String,
        col7: String,
        col8: String,
        col9: Vec<u8>,
        col10: Vec<u8>
    }

    #[test]
    fn strings_infer_to_semantically_correct_types() {
        let conn = connection();
        let inferred_strings = InferredStrings {
            col1: "Hello".into(),
            col2: "Hello".into(),
            col3: "Hello".into(),
            col4: "Hello".into(),
            col5: "Hello".into(),
            col6: "Hello".into(),
            col7: "Hello".into(),
            col8: "Hello".into(),
            col9: vec![1, 2, 3],
            col10: vec![1, 2, 3],
        };
        insert(&inferred_strings).into(infer_all_the_strings::table)
            .execute(&conn).unwrap();

        assert_eq!(Ok(vec![inferred_strings]), infer_all_the_strings::table.load(&conn));
    }

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[table_name="infer_all_the_floats"]
    struct InferredFloats {
        col1: f32,
        col2: f32,
        col3: f64,
        col4: f64,
        col5: f64,
        col6: f64,
    }

    #[test]
    fn floats_infer_to_semantically_correct_types() {
        let conn = connection();
        let inferred_floats = InferredFloats {
            col1: 1.0,
            col2: 2.0,
            col3: 3.0,
            col4: 4.0,
            col5: 5.0,
            col6: 6.0,
        };
        insert(&inferred_floats).into(infer_all_the_floats::table)
            .execute(&conn).unwrap();

        assert_eq!(Ok(vec![inferred_floats]), infer_all_the_floats::table.load(&conn));
    }
}
