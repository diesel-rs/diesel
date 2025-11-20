use crate::diesel::ExpressionMethods;
use diesel::sql_types::*;
use diesel::IntoSql;
use diesel::RunQueryDsl;
#[cfg(feature = "mysql")]
use std::str::FromStr;

macro_rules! test_cast {
    ($test_name: ident, $input_value: expr => $rust_input_ty: ty, $input_ty: ty => $target_ty: ty, $rust_value: expr => $rust_ty: ty, $cast: ident) => {
        #[diesel_test_helper::test]
        /// Test Text -> Int4
        fn $test_name() {
            let conn = &mut crate::schema::connection();

            let from: $rust_input_ty = $input_value;
            let to: $rust_ty = $rust_value;

            let result = diesel::select(from.into_sql::<$input_ty>().$cast::<$target_ty>())
                .get_result::<$rust_ty>(conn)
                .unwrap();
            assert_eq!(result, to);

            let from = None::<$rust_input_ty>;
            let to = None::<$rust_ty>;

            let result = diesel::select(
                from.into_sql::<Nullable<$input_ty>>()
                    .$cast::<Nullable<$target_ty>>(),
            )
            .get_result::<Option<$rust_ty>>(conn)
            .unwrap();

            assert_eq!(result, to);
        }
    };
}

macro_rules! test_fallible_cast {
   ($test_name: ident, $input_value: expr => $rust_input_ty: ty, $input_ty: ty => $target_ty: ty, $rust_value: expr => $rust_ty: ty) => {
       test_cast!($test_name, $input_value => $rust_input_ty, $input_ty => $target_ty, $rust_value => $rust_ty, fallible_cast);
   }
}

macro_rules! test_infallible_cast {
   ($test_name: ident, $input_value: expr => $rust_input_ty: ty, $input_ty: ty => $target_ty: ty, $rust_value: expr => $rust_ty: ty) => {
       test_cast!($test_name, $input_value => $rust_input_ty, $input_ty => $target_ty, $rust_value => $rust_ty, cast);
   }
}

mod fallible_cast {
    use super::*;

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    test_fallible_cast!(int8_to_int4, 3 => i64, Int8 => Int4, 3 => i32);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(float8_to_int4, 3.1 => f64, Float8 => Int4, 3 => i32);
    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    test_fallible_cast!(text_to_int4, "3" => &str, Text => Int4, 3 => i32);
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    test_fallible_cast!(decimal_to_int4, "3.1".parse().unwrap() => bigdecimal::BigDecimal, Decimal => Int4, 3 => i32);
    test_fallible_cast!(text_to_int8, "3" => &str, Text => Int8, 3 => i64);
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    test_fallible_cast!(decimal_to_int8, "3.1".parse().unwrap() => bigdecimal::BigDecimal, Decimal => Int8, 3 => i64);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(float8_to_float4, 3.1 => f64, Float8 => Float4, 3.1 => f32);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_float4, "3.1" => &str, Text => Float4, 3.1 => f32);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(decimal_to_float4, "3.1".parse().unwrap() => bigdecimal::BigDecimal, Decimal => Float8, 3.1 => f64);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_float8, "3.1" => &str, Text => Float4, 3.1 => f32);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(decimal_to_float8, "3.1".parse().unwrap() => bigdecimal::BigDecimal, Decimal => Float8, 3.1 => f64);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_json, "[1, 2, 3]" => &str, Text => Json, serde_json::json!([1,2,3]) => serde_json::Value);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_jsonb, "[1, 2, 3]" => &str, Text => Jsonb, serde_json::json!([1,2,3]) => serde_json::Value);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_bool, "true" => &str, Text => Bool, true => bool);
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    test_fallible_cast!(text_to_date, "2025-09-19" => &str, Text => Date, chrono::NaiveDate::from_ymd_opt(2025, 9, 19).unwrap() => chrono::NaiveDate);
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    test_fallible_cast!(text_to_time, "10:14:42" => &str, Text => Time, chrono::NaiveTime::from_hms_opt(10, 14, 42).unwrap() => chrono::NaiveTime);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_decimal, "3.1" => &str, Text => Decimal, "3.1".parse().unwrap() => bigdecimal::BigDecimal);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_uuid, "9d755438-5ad5-42b8-bc6a-a15ace143975" => &str, Text => Uuid, "9d755438-5ad5-42b8-bc6a-a15ace143975".parse().unwrap() => uuid::Uuid);
    #[cfg(feature = "mysql")]
    test_fallible_cast!(text_to_datetime, "2025-09-19 10:21:42" => &str, Text => Datetime, chrono::NaiveDateTime::from_str("2025-09-19T10:21:42").unwrap() => chrono::NaiveDateTime);

    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetv4_without_prefix, String::from("1.1.1.1") => String, Text => Inet, "1.1.1.1/32".parse().unwrap() => ipnet::IpNet);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetv4, String::from("1.1.1.1/32") => String, Text => Inet, "1.1.1.1/32".parse().unwrap() => ipnet::IpNet);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetv6_without_prefix, String::from("1::1") => String, Text => Inet, "1::1/128".parse().unwrap() => ipnet::IpNet);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetv6, String::from("1::1/128") => String, Text => Inet, "1::1/128".parse().unwrap() => ipnet::IpNet);

    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetv4_without_prefix, String::from("1.1.1.1") => String, Text => Cidr, "1.1.1.1/32".parse().unwrap() => ipnet::IpNet);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetv4, String::from("1.1.1.1/32") => String, Text => Cidr, "1.1.1.1/32".parse().unwrap() => ipnet::IpNet);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetv6_without_prefix, String::from("1::1") => String, Text => Cidr, "1::1/128".parse().unwrap() => ipnet::IpNet);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetv6, String::from("1::1/128") => String, Text => Cidr, "1::1/128".parse().unwrap() => ipnet::IpNet);

    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetworkv4_without_prefix, String::from("1.1.1.1") => String, Text => Inet, "1.1.1.1/32".parse().unwrap() => ipnetwork::IpNetwork);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetworkv4, String::from("1.1.1.1/32") => String, Text => Inet, "1.1.1.1/32".parse().unwrap() => ipnetwork::IpNetwork);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetworkv6_without_prefix, String::from("1::1") => String, Text => Inet, "1::1/128".parse().unwrap() => ipnetwork::IpNetwork);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_inet_ipnetworkv6, String::from("1::1/128") => String, Text => Inet, "1::1/128".parse().unwrap() => ipnetwork::IpNetwork);

    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetworkv4_without_prefix, String::from("1.1.1.1") => String, Text => Cidr, "1.1.1.1/32".parse().unwrap() => ipnetwork::IpNetwork);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetworkv4, String::from("1.1.1.1/32") => String, Text => Cidr, "1.1.1.1/32".parse().unwrap() => ipnetwork::IpNetwork);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetworkv6_without_prefix, String::from("1::1") => String, Text => Cidr, "1::1/128".parse().unwrap() => ipnetwork::IpNetwork);
    #[cfg(feature = "postgres")]
    test_fallible_cast!(text_to_cidr_ipnetworkv6, String::from("1::1/128") => String, Text => Cidr, "1::1/128".parse().unwrap() => ipnetwork::IpNetwork);
}

mod infallible_cast {
    use super::*;
    #[cfg(feature = "postgres")]
    test_infallible_cast!(int4_to_bool, 4 => i32, Integer => Bool, true => bool);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(bool_to_int4, true => bool, Bool => Integer, 1 => i32);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(float4_to_int4, 3.1 => f32, Float => Integer, 3 => i32);

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    test_infallible_cast!(int4_to_bigint, 4 => i32, Integer => BigInt, 4 => i64);

    #[cfg(feature = "postgres")]
    test_infallible_cast!(float4_to_int8, 3.1 => f32, Float => BigInt, 3 => i64);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(float8_to_int8, 3.1 => f64, Double => BigInt, 3 => i64);

    #[cfg(feature = "postgres")]
    test_infallible_cast!(int4_to_float4, 3 => i32, Integer => Float, 3.0 => f32);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(int8_to_float4, 3 => i64, BigInt => Float, 3.0 => f32);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(int4_to_float8, 3 => i32, Integer => Double, 3.0 => f64);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(int8_to_float8, 3 => i64, BigInt => Double, 3.0 => f64);

    #[cfg(feature = "postgres")]
    test_infallible_cast!(bool_to_text, true => bool, Bool => Text, String::from("true") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(float4_to_text, 3.1 => f32, Float4 => Text, String::from("3.1") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(float8_to_text, 3.1 => f64, Float8 => Text, String::from("3.1") => String);
    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    test_infallible_cast!(int4_to_text, 3 => i32, Integer => Text, String::from("3") => String);
    test_infallible_cast!(int8_to_text, 3 => i64, BigInt => Text, String::from("3") => String);
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    test_infallible_cast!(date_to_text, chrono::NaiveDate::from_ymd_opt(2025, 9, 19).unwrap() => chrono::NaiveDate, Date => Text, String::from("2025-09-19") => String);
    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    test_infallible_cast!(json_to_text, serde_json::json!([1,2,3]) => serde_json::Value, Json => Text, String::from("[1,2,3]") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(jsonb_to_text, serde_json::json!([1,2,3]) => serde_json::Value, Jsonb => Text, String::from("[1, 2, 3]") => String);
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    test_infallible_cast!(time_to_text, chrono::NaiveTime::from_hms_opt(10, 1, 42).unwrap() => chrono::NaiveTime, Time => Text, String::from("10:01:42") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(json_to_jsonb, serde_json::json!([1,2,3]) => serde_json::Value, Json => Jsonb, serde_json::json!([1,2,3]) => serde_json::Value);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(jsonb_to_json, serde_json::json!([1,2,3]) => serde_json::Value, Jsonb => Json, serde_json::json!([1,2,3]) => serde_json::Value);

    #[cfg(feature = "postgres")]
    test_infallible_cast!(text_to_uuid, "9d755438-5ad5-42b8-bc6a-a15ace143975".parse().unwrap() => uuid::Uuid, Uuid => Text, "9d755438-5ad5-42b8-bc6a-a15ace143975".into() => String);
    #[cfg(feature = "mysql")]
    test_infallible_cast!(text_to_datetime, chrono::NaiveDateTime::from_str("2025-09-19T10:21:42").unwrap() => chrono::NaiveDateTime, Datetime => Text, "2025-09-19 10:21:42".into() => String);

    #[cfg(any(feature = "postgres", feature = "mysql"))]
    test_infallible_cast!(int4_to_decimal, 1 => i32, Int4 => Decimal, "1".parse().unwrap() => bigdecimal::BigDecimal);
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    test_infallible_cast!(int8_to_decimal, 1 => i64, Int8 => Decimal, "1".parse().unwrap() => bigdecimal::BigDecimal);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(float4_to_decimal, 1.1 => f32, Float4 => Decimal, "1.1".parse().unwrap() => bigdecimal::BigDecimal);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(float8_to_decimal, 1.1 => f64, Float8 => Decimal, "1.1".parse().unwrap() => bigdecimal::BigDecimal);

    #[cfg(feature = "postgres")]
    test_infallible_cast!(inet_ipnet_to_text, "1.1.1.1/32".parse().unwrap() => ipnet::IpNet, Inet => Text, String::from("1.1.1.1/32") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(cidr_ipnet_to_text, "1.1.1.1/32".parse().unwrap() => ipnet::IpNet, Cidr => Text, String::from("1.1.1.1/32") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(cidr_ipnet_to_inet_ipnet, "192.168.1.0/24".parse().unwrap() => ipnet::IpNet, Cidr => Inet, "192.168.1.0/24".parse().unwrap() => ipnet::IpNet);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(inet_ipnet_to_cidr_ipnet, "192.168.1.1/24".parse().unwrap() => ipnet::IpNet, Inet => Cidr, "192.168.1.0/24".parse().unwrap() => ipnet::IpNet);

    #[cfg(feature = "postgres")]
    test_infallible_cast!(inet_ipnetwork_to_text, "1.1.1.1/32".parse().unwrap() => ipnetwork::IpNetwork, Inet => Text, String::from("1.1.1.1/32") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(cidr_ipnetwork_to_text, "1.1.1.1/32".parse().unwrap() => ipnetwork::IpNetwork, Cidr => Text, String::from("1.1.1.1/32") => String);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(cidr_ipnetwork_to_inet_ipnetwork, "192.168.1.0/24".parse().unwrap() => ipnetwork::IpNetwork, Cidr => Inet, "192.168.1.0/24".parse().unwrap() => ipnetwork::IpNetwork);
    #[cfg(feature = "postgres")]
    test_infallible_cast!(inet_ipnetwork_to_cidr_ipnetwork, "192.168.1.1/24".parse().unwrap() => ipnetwork::IpNetwork, Inet => Cidr, "192.168.1.0/24".parse().unwrap() => ipnetwork::IpNetwork);
}
