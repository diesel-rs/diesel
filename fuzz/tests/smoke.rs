use diesel_fuzz::{pg, sqlite};
use std::num::NonZeroU32;

#[test]
fn every_pg_case_decodes_without_panicking() {
    let oid = NonZeroU32::MIN;
    for selector in 0..pg::CASES.len() {
        let selector = u8::try_from(selector).expect("under 256 cases");
        for bytes in [&[][..], &[0x00], &[0xFF; 4], &[0x7F; 16], &[0xAA; 64]] {
            pg::decode_case(selector, oid, bytes);
        }
    }
}

#[test]
fn every_sqlite_case_decodes_without_panicking() {
    for selector in 0..sqlite::CASES.len() {
        let selector = u8::try_from(selector).expect("under 256 cases");
        for kind in 0..4 {
            for bytes in [
                &[][..],
                &[0x00],
                &[0xFF; 8],
                b"2024-06-15 10:30:45",
                b"1.5e3",
            ] {
                sqlite::decode_case(selector, kind, bytes);
            }
        }
    }
}

#[test]
fn documents_survive_the_round_trip() {
    sqlite::with_conn(|conn| {
        for document in [
            "null",
            "true",
            "false",
            "0",
            "-1",
            "1.5",
            "1e-7",
            "\"\"",
            "\"a\"",
            "[]",
            "{}",
            "[null,true,1,\"a\",[],{}]",
            "{\"a\":{\"b\":[1,2,3]}}",
        ] {
            let value: serde_json::Value = serde_json::from_str(document).expect("valid json");
            sqlite::roundtrip_jsonb(conn, &value).expect(document);
            sqlite::roundtrip_json(conn, &value).expect(document);
        }
    });
}

#[test]
fn a_decoded_blob_is_the_one_sqlite_calls_valid() {
    sqlite::with_conn(|conn| {
        assert_eq!(sqlite::jsonb_valid(conn, &[0x00]), Ok(true));
        assert_eq!(sqlite::jsonb_valid(conn, &[0xFF]), Ok(false));
        assert!(sqlite::decode_jsonb(conn, &[0xFF]).is_err());
    });
}
