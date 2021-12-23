extern crate chrono;

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::chrono::*;
    use crate::schema::*;
    use diesel::*;

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[diesel(table_name = infer_all_the_ints)]
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
        let conn = &mut connection();
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
        insert_into(infer_all_the_ints::table)
            .values(&inferred_ints)
            .execute(conn)
            .unwrap();

        assert_eq!(
            Ok(vec![inferred_ints]),
            infer_all_the_ints::table.load(conn)
        );
    }

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[diesel(table_name = infer_all_the_bools)]
    struct InferredBools {
        col1: bool,
        col2: bool,
        col3: bool,
        col4: bool,
    }

    #[test]
    fn bool_types_infer_to_bool() {
        let conn = &mut connection();
        let inferred_bools = InferredBools {
            col1: true,
            col2: true,
            col3: false,
            col4: false,
        };
        insert_into(infer_all_the_bools::table)
            .values(&inferred_bools)
            .execute(conn)
            .unwrap();

        assert_eq!(
            Ok(vec![inferred_bools]),
            infer_all_the_bools::table.load(conn)
        );
    }

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[diesel(table_name = infer_all_the_strings)]
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
        col10: Vec<u8>,
    }

    #[test]
    fn strings_infer_to_semantically_correct_types() {
        let conn = &mut connection();
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
        insert_into(infer_all_the_strings::table)
            .values(&inferred_strings)
            .execute(conn)
            .unwrap();

        assert_eq!(
            Ok(vec![inferred_strings]),
            infer_all_the_strings::table.load(conn)
        );
    }

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[diesel(table_name = infer_all_the_floats)]
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
        let conn = &mut connection();
        let inferred_floats = InferredFloats {
            col1: 1.0,
            col2: 2.0,
            col3: 3.0,
            col4: 4.0,
            col5: 5.0,
            col6: 6.0,
        };
        insert_into(infer_all_the_floats::table)
            .values(&inferred_floats)
            .execute(conn)
            .unwrap();

        assert_eq!(
            Ok(vec![inferred_floats]),
            infer_all_the_floats::table.load(conn)
        );
    }

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[diesel(table_name = infer_all_the_datetime_types)]
    struct InferredDatetimeTypes {
        dt: NaiveDateTime,
        date: NaiveDate,
        time: NaiveTime,
        timestamp: NaiveDateTime,
    }

    #[test]
    fn datetime_types_are_correctly_inferred() {
        let conn = &mut connection();

        let dt = NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11);
        let inferred_datetime_types = InferredDatetimeTypes {
            dt: dt,
            date: dt.date(),
            time: dt.time(),
            timestamp: dt,
        };

        insert_into(infer_all_the_datetime_types::table)
            .values(&inferred_datetime_types)
            .execute(conn)
            .unwrap();

        assert_eq!(
            Ok(vec![inferred_datetime_types]),
            infer_all_the_datetime_types::table.load(conn)
        );
    }
}

#[cfg(feature = "postgres")]
mod postgres {
    use super::chrono::*;
    use crate::schema::*;
    use diesel::data_types::PgNumeric;
    use diesel::*;
    use std::collections::Bound;

    #[derive(Queryable, PartialEq, Debug, Insertable)]
    #[diesel(table_name = all_the_ranges)]
    struct InferredRanges {
        int4: (Bound<i32>, Bound<i32>),
        int8: (Bound<i64>, Bound<i64>),
        num: (Bound<PgNumeric>, Bound<PgNumeric>),
        ts: (Bound<NaiveDateTime>, Bound<NaiveDateTime>),
        tstz: (Bound<DateTime<Utc>>, Bound<DateTime<Utc>>),
        date: (Bound<NaiveDate>, Bound<NaiveDate>),
    }

    #[test]
    fn ranges_are_correctly_inferred() {
        let conn = &mut connection();
        let numeric = PgNumeric::Positive {
            weight: 1,
            scale: 1,
            digits: vec![1],
        };
        let dt = NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11);

        let inferred_ranges = InferredRanges {
            int4: (Bound::Included(5), Bound::Excluded(12)),
            int8: (Bound::Included(5), Bound::Excluded(13)),
            num: (Bound::Included(numeric), Bound::Unbounded),
            ts: (Bound::Included(dt), Bound::Unbounded),
            tstz: (
                Bound::Unbounded,
                Bound::Excluded(DateTime::<Utc>::from_utc(dt, Utc)),
            ),
            date: (Bound::Included(dt.date()), Bound::Unbounded),
        };

        insert_into(all_the_ranges::table)
            .values(&inferred_ranges)
            .execute(conn)
            .unwrap();

        assert_eq!(Ok(vec![inferred_ranges]), all_the_ranges::table.load(conn));
    }
}

#[cfg(feature = "mysql")]
mod mysql {
    use crate::schema::*;
    use diesel::*;

    #[derive(Insertable)]
    #[diesel(table_name = all_the_blobs)]
    struct InferredBlobs<'a> {
        id: i32,
        tiny: &'a [u8],
        normal: &'a [u8],
        medium: &'a [u8],
        big: &'a [u8],
    }

    #[derive(Queryable, Debug, PartialEq)]
    struct Blobs {
        id: i32,
        tiny: Vec<u8>,
        normal: Vec<u8>,
        medium: Vec<u8>,
        big: Vec<u8>,
    }

    #[test]
    fn blobs_are_correctly_inferred() {
        let conn = &mut connection();
        let inferred_blobs = InferredBlobs {
            id: 0,
            tiny: &[0x01],
            normal: &[0x02],
            medium: &[0x03],
            big: &[0x04],
        };

        let blobs = Blobs {
            id: 0,
            tiny: vec![0x01],
            normal: vec![0x02],
            medium: vec![0x03],
            big: vec![0x04],
        };

        insert_into(all_the_blobs::table)
            .values(&inferred_blobs)
            .execute(conn)
            .unwrap();
        assert_eq!(Ok(vec![blobs]), all_the_blobs::table.load(conn));
    }
}

#[test]
fn columns_named_as_reserved_keywords_are_renamed() {
    use crate::schema::*;
    use diesel::*;

    #[derive(Queryable, Insertable, Debug, PartialEq)]
    #[diesel(table_name = with_keywords)]
    struct WithKeywords {
        fn_: i32,
        let_: i32,
        extern_: i32,
    }

    let value = WithKeywords {
        fn_: 1,
        let_: 42,
        extern_: 51,
    };

    let conn = &mut connection();
    insert_into(with_keywords::table)
        .values(&value)
        .execute(conn)
        .unwrap();
    assert_eq!(Ok(vec![value]), with_keywords::table.load(conn));
}
