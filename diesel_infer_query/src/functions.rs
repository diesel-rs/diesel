// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Which functions of a backend are aggregates
//!
//! A query without `GROUP BY` that calls an aggregate returns one row even for empty
//! input, so knowing the aggregates matters for the nullability of the columns next to
//! them. SQLite applications, MySQL and MariaDB loadable functions, and MariaDB stored
//! functions can add aggregates of any name that is not taken by a built-in function,
//! so only the built-in functions of the backend a definition comes from are known not
//! to aggregate.

use crate::backend::Backend;
use sqlparser::ast::{Function, FunctionArguments, ObjectNamePart};

/// Built-in aggregate functions of SQLite, MySQL, and MariaDB, the backends that allow
/// columns outside aggregates in a query without `GROUP BY`, including the ones of
/// SQLite's percentile extension
#[rustfmt::skip]
const AGGREGATE_FUNCTIONS: &[&str] = &[
    "avg", "bit_and", "bit_or", "bit_xor", "count", "group_concat", "json_arrayagg",
    "json_group_array", "json_group_object", "json_objectagg", "jsonb_group_array",
    "jsonb_group_object", "max", "median", "min", "percentile", "percentile_cont",
    "percentile_disc", "st_collect", "std", "stddev", "stddev_pop", "stddev_samp",
    "string_agg", "sum", "total", "var_pop", "var_samp", "variance",
];

/// The built-in scalar functions of SQLite and the number of arguments they take, as
/// `PRAGMA function_list` of SQLite 3.50 with math functions and of the bundled SQLite
/// 3.53 reports them: `N` for exactly `N`, `-1` for any number, and `-2 - N` for at
/// least `N`
///
/// SQLite lets applications replace built-in functions too, but replacing one with an
/// aggregate taking the same number of arguments would break every ordinary use of it.
#[rustfmt::skip]
const SQLITE_SCALAR_FUNCTIONS: &[(&str, i8)] = &[
    ("abs", 1), ("acos", 1), ("acosh", 1), ("asin", 1), ("asinh", 1), ("atan", 1),
    ("atan2", 2), ("atanh", 1), ("ceil", 1), ("ceiling", 1), ("changes", 0), ("char", -1),
    ("coalesce", -4), ("concat", -3), ("concat_ws", -4), ("cos", 1), ("cosh", 1),
    ("current_date", 0), ("current_time", 0), ("current_timestamp", 0), ("date", -1),
    ("datetime", -1), ("degrees", 1), ("exp", 1), ("floor", 1), ("format", -1), ("glob", 2),
    ("hex", 1), ("if", -4), ("ifnull", 2), ("iif", -4), ("instr", 2), ("json", 1),
    ("json_array", -1), ("json_array_insert", -1), ("json_array_length", 1),
    ("json_array_length", 2), ("json_error_position", 1), ("json_extract", -1),
    ("json_insert", -1), ("json_object", -1), ("json_patch", 2), ("json_pretty", 1),
    ("json_pretty", 2), ("json_quote", 1), ("json_remove", -1), ("json_replace", -1),
    ("json_set", -1), ("json_type", 1), ("json_type", 2), ("json_valid", 1),
    ("json_valid", 2), ("jsonb", 1), ("jsonb_array", -1), ("jsonb_array_insert", -1),
    ("jsonb_extract", -1), ("jsonb_insert", -1), ("jsonb_object", -1), ("jsonb_patch", 2),
    ("jsonb_remove", -1), ("jsonb_replace", -1), ("jsonb_set", -1), ("julianday", -1),
    ("last_insert_rowid", 0), ("length", 1), ("like", 2), ("like", 3), ("likelihood", 2),
    ("likely", 1), ("ln", 1), ("load_extension", 1), ("load_extension", 2), ("log", 1),
    ("log", 2), ("log10", 1), ("log2", 1), ("lower", 1), ("ltrim", 1), ("ltrim", 2),
    ("max", -3), ("min", -3), ("mod", 2), ("nullif", 2), ("octet_length", 1), ("pi", 0),
    ("pow", 2), ("power", 2), ("printf", -1), ("quote", 1), ("radians", 1), ("random", 0),
    ("randomblob", 1), ("replace", 3), ("round", 1), ("round", 2), ("rtrim", 1),
    ("rtrim", 2), ("sign", 1), ("sin", 1), ("sinh", 1), ("soundex", 1),
    ("sqlite_compileoption_get", 1), ("sqlite_compileoption_used", 1), ("sqlite_log", 2),
    ("sqlite_source_id", 0), ("sqlite_version", 0), ("sqrt", 1), ("strftime", -1),
    ("substr", 2), ("substr", 3), ("substring", 2), ("substring", 3), ("subtype", 1),
    ("tan", 1), ("tanh", 1), ("time", -1), ("timediff", 2), ("total_changes", 0),
    ("trim", 1), ("trim", 2), ("trunc", 1), ("typeof", 1), ("unhex", 1), ("unhex", 2),
    ("unicode", 1), ("unistr", 1), ("unistr_quote", 1), ("unixepoch", -1), ("unlikely", 1),
    ("upper", 1), ("zeroblob", 1),
];

/// The native functions of MySQL 8.4 outside its aggregates, from the `func_array` of
/// `sql/item_create.cc` and the functions its grammar spells out, without the internal
/// ones of the data dictionary
///
/// A native function always takes precedence over a loadable function of the same name.
#[rustfmt::skip]
const MYSQL_SCALAR_FUNCTIONS: &[&str] = &[
    "abs", "acos", "adddate", "addtime", "aes_decrypt", "aes_encrypt", "any_value", "ascii",
    "asin", "atan", "atan2", "benchmark", "bin", "bin_to_uuid", "bit_count", "bit_length",
    "ceil", "ceiling", "char", "char_length", "character", "character_length", "charset",
    "coalesce", "coercibility", "collation", "compress", "concat", "concat_ws",
    "connection_id", "conv", "convert_tz", "cos", "cot", "crc32", "curdate", "current_date",
    "current_role", "current_time", "current_timestamp", "current_user", "curtime",
    "database", "date", "date_add", "date_format", "date_sub", "datediff", "day", "dayname",
    "dayofmonth", "dayofweek", "dayofyear", "degrees", "elt", "exp", "export_set",
    "extract", "extractvalue", "field", "find_in_set", "floor", "format", "format_bytes",
    "format_pico_time", "found_rows", "from_base64", "from_days", "from_unixtime",
    "geomcollection", "geometrycollection", "get_format", "get_lock", "greatest",
    "gtid_subset", "gtid_subtract", "hex", "hour", "icu_version", "if", "ifnull",
    "inet6_aton", "inet6_ntoa", "inet_aton", "inet_ntoa", "insert", "instr", "interval",
    "is_free_lock", "is_ipv4", "is_ipv4_compat", "is_ipv4_mapped", "is_ipv6",
    "is_used_lock", "is_uuid", "isnull", "json_array", "json_array_append",
    "json_array_insert", "json_contains", "json_contains_path", "json_depth",
    "json_extract", "json_insert", "json_keys", "json_length", "json_merge",
    "json_merge_patch", "json_merge_preserve", "json_object", "json_overlaps",
    "json_pretty", "json_quote", "json_remove", "json_replace", "json_schema_valid",
    "json_schema_validation_report", "json_search", "json_set", "json_storage_free",
    "json_storage_size", "json_type", "json_unquote", "json_valid", "json_value",
    "last_day", "last_insert_id", "lcase", "least", "left", "length", "like_range_max",
    "like_range_min", "linestring", "ln", "load_file", "localtime", "localtimestamp",
    "locate", "log", "log10", "log2", "lower", "lpad", "ltrim", "make_set", "makedate",
    "maketime", "master_pos_wait", "mbrcontains", "mbrcoveredby", "mbrcovers",
    "mbrdisjoint", "mbrequals", "mbrintersects", "mbroverlaps", "mbrtouches", "mbrwithin",
    "md5", "microsecond", "mid", "minute", "mod", "month", "monthname", "multilinestring",
    "multipoint", "multipolygon", "name_const", "now", "nullif", "oct", "octet_length",
    "ord", "period_add", "period_diff", "pi", "point", "polygon", "position", "pow",
    "power", "ps_current_thread_id", "ps_thread_id", "quarter", "quote", "radians", "rand",
    "random_bytes", "regexp_instr", "regexp_like", "regexp_replace", "regexp_substr",
    "release_all_locks", "release_lock", "repeat", "replace", "reverse", "right",
    "roles_graphml", "round", "row_count", "rpad", "rtrim", "schema", "sec_to_time",
    "second", "session_user", "sha", "sha1", "sha2", "sign", "sin", "sleep", "soundex",
    "source_pos_wait", "space", "sqrt", "st_area", "st_asbinary", "st_asgeojson",
    "st_astext", "st_aswkb", "st_aswkt", "st_buffer", "st_buffer_strategy", "st_centroid",
    "st_contains", "st_convexhull", "st_crosses", "st_difference", "st_dimension",
    "st_disjoint", "st_distance", "st_distance_sphere", "st_endpoint", "st_envelope",
    "st_equals", "st_exteriorring", "st_frechetdistance", "st_geohash",
    "st_geomcollfromtext", "st_geomcollfromtxt", "st_geomcollfromwkb",
    "st_geometrycollectionfromtext", "st_geometrycollectionfromwkb", "st_geometryfromtext",
    "st_geometryfromwkb", "st_geometryn", "st_geometrytype", "st_geomfromgeojson",
    "st_geomfromtext", "st_geomfromwkb", "st_hausdorffdistance", "st_interiorringn",
    "st_intersection", "st_intersects", "st_isclosed", "st_isempty", "st_issimple",
    "st_isvalid", "st_latfromgeohash", "st_latitude", "st_length", "st_linefromtext",
    "st_linefromwkb", "st_lineinterpolatepoint", "st_lineinterpolatepoints",
    "st_linestringfromtext", "st_linestringfromwkb", "st_longfromgeohash", "st_longitude",
    "st_makeenvelope", "st_mlinefromtext", "st_mlinefromwkb", "st_mpointfromtext",
    "st_mpointfromwkb", "st_mpolyfromtext", "st_mpolyfromwkb", "st_multilinestringfromtext",
    "st_multilinestringfromwkb", "st_multipointfromtext", "st_multipointfromwkb",
    "st_multipolygonfromtext", "st_multipolygonfromwkb", "st_numgeometries",
    "st_numinteriorring", "st_numinteriorrings", "st_numpoints", "st_overlaps",
    "st_pointatdistance", "st_pointfromgeohash", "st_pointfromtext", "st_pointfromwkb",
    "st_pointn", "st_polyfromtext", "st_polyfromwkb", "st_polygonfromtext",
    "st_polygonfromwkb", "st_simplify", "st_srid", "st_startpoint", "st_swapxy",
    "st_symdifference", "st_touches", "st_transform", "st_union", "st_validate",
    "st_within", "st_x", "st_y", "statement_digest", "statement_digest_text", "str_to_date",
    "strcmp", "subdate", "substr", "substring", "substring_index", "subtime", "sysdate",
    "system_user", "tan", "time", "time_format", "time_to_sec", "timediff", "timestamp",
    "timestampadd", "timestampdiff", "to_base64", "to_days", "to_seconds", "trim",
    "truncate", "ucase", "uncompress", "uncompressed_length", "unhex", "unix_timestamp",
    "updatexml", "upper", "user", "utc_date", "utc_time", "utc_timestamp", "uuid",
    "uuid_short", "uuid_to_bin", "validate_password_strength", "version",
    "wait_for_executed_gtid_set", "week", "weekday", "weekofyear", "weight_string", "year",
    "yearweek",
];

/// The native functions of MariaDB 11.4 outside its aggregates, from the `func_array`
/// of `sql/item_create.cc` and the functions its grammar spells out, without the ones
/// of its Oracle mode and of Galera
///
/// A native function always takes precedence over a loadable or stored function of the
/// same name.
#[rustfmt::skip]
const MARIADB_SCALAR_FUNCTIONS: &[&str] = &[
    "abs", "acos", "add_months", "adddate", "addtime", "aes_decrypt", "aes_encrypt",
    "ascii", "asin", "atan", "atan2", "benchmark", "bin", "bit_count", "bit_length", "ceil",
    "ceiling", "char", "char_length", "character", "character_length", "charset", "chr",
    "coalesce", "coercibility", "collation", "column_add", "column_check", "column_create",
    "column_delete", "column_exists", "column_get", "column_json", "column_list",
    "compress", "concat", "concat_ws", "connection_id", "conv", "convert_tz", "cos", "cot",
    "crc32", "crc32c", "curdate", "current_date", "current_role", "current_time",
    "current_timestamp", "current_user", "curtime", "database", "date", "date_add",
    "date_format", "date_sub", "datediff", "day", "dayname", "dayofmonth", "dayofweek",
    "dayofyear", "decode", "degrees", "des_decrypt", "des_encrypt", "elt", "encode",
    "encrypt", "exp", "export_set", "extract", "extractvalue", "field", "find_in_set",
    "floor", "format", "format_pico_time", "found_rows", "from_base64", "from_days",
    "from_unixtime", "get_format", "get_lock", "greatest", "hex", "hour", "if", "ifnull",
    "insert", "instr", "interval", "is_free_lock", "is_used_lock", "isnull", "json_array",
    "json_array_append", "json_array_insert", "json_array_intersect", "json_compact",
    "json_contains", "json_contains_path", "json_depth", "json_detailed", "json_equals",
    "json_exists", "json_extract", "json_insert", "json_key_value", "json_keys",
    "json_length", "json_loose", "json_merge", "json_merge_patch", "json_merge_preserve",
    "json_normalize", "json_object", "json_object_filter_keys", "json_object_to_array",
    "json_overlaps", "json_pretty", "json_query", "json_quote", "json_remove",
    "json_replace", "json_schema_valid", "json_search", "json_set", "json_type",
    "json_unquote", "json_valid", "json_value", "kdf", "last_day", "last_insert_id",
    "last_value", "lcase", "least", "left", "length", "lengthb", "like_range_max",
    "like_range_min", "ln", "load_file", "localtime", "localtimestamp", "locate", "log",
    "log10", "log2", "lower", "lpad", "ltrim", "make_set", "makedate", "maketime",
    "master_gtid_wait", "master_pos_wait", "md5", "microsecond", "mid", "minute", "mod",
    "month", "monthname", "name_const", "natural_sort_key", "now", "nullif", "nvl", "nvl2",
    "oct", "octet_length", "old_password", "ord", "password", "period_add", "period_diff",
    "pi", "position", "pow", "power", "quarter", "quote", "radians", "rand", "random_bytes",
    "regexp_instr", "regexp_replace", "regexp_substr", "release_all_locks", "release_lock",
    "repeat", "replace", "reverse", "right", "round", "row_count", "rownum", "rpad",
    "rtrim", "schema", "schemas", "sec_to_time", "second", "session_user", "sformat", "sha",
    "sha1", "sha2", "sign", "sin", "sleep", "soundex", "space", "sqrt", "str_to_date",
    "strcmp", "subdate", "substr", "substring", "substring_index", "subtime", "sysdate",
    "system_user", "tan", "time", "time_format", "time_to_sec", "timediff", "timestamp",
    "timestampadd", "timestampdiff", "to_base64", "to_char", "to_days", "to_seconds",
    "trim", "truncate", "ucase", "uncompress", "uncompressed_length", "unhex",
    "unix_timestamp", "updatexml", "upper", "user", "utc_date", "utc_time", "utc_timestamp",
    "uuid_short", "version", "week", "weekday", "weekofyear", "weight_string", "year",
    "yearweek",
];

/// Whether a call of a function is an aggregate
pub(crate) enum FunctionKind {
    Aggregate,
    Scalar,
    /// A function the backend does not define itself, which can be an aggregate
    Unknown,
}

/// Classify a call of `function` in a definition of `backend`
pub(crate) fn function_kind(function: &Function, backend: Backend) -> FunctionKind {
    let [ObjectNamePart::Identifier(name)] = function.name.0.as_slice() else {
        // a name qualified with a schema refers to a stored function, which MariaDB
        // lets users define as an aggregate
        return FunctionKind::Unknown;
    };
    let is = |candidate: &&str| name.value.eq_ignore_ascii_case(candidate);
    let arguments = match &function.args {
        FunctionArguments::None => 0,
        FunctionArguments::List(list) => list.args.len(),
        FunctionArguments::Subquery(_) => return FunctionKind::Unknown,
    };
    // SQLite's `min()` and `max()` with more than one argument are scalar functions
    let scalar_min_or_max = ["min", "max"].iter().any(is) && arguments != 1;
    if AGGREGATE_FUNCTIONS.iter().any(is) && !scalar_min_or_max {
        return FunctionKind::Aggregate;
    }
    let built_in = match backend {
        Backend::Sqlite => SQLITE_SCALAR_FUNCTIONS
            .iter()
            .any(|(candidate, takes)| is(candidate) && accepts(*takes, arguments)),
        Backend::Mysql => MYSQL_SCALAR_FUNCTIONS.iter().any(is),
        Backend::Mariadb => MARIADB_SCALAR_FUNCTIONS.iter().any(is),
        // PostgreSQL rejects columns outside the aggregates of a query that aggregates
        // without `GROUP BY`, so no view it stores needs this
        Backend::Pg => false,
    };
    if built_in {
        FunctionKind::Scalar
    } else {
        FunctionKind::Unknown
    }
}

/// Whether a function taking `takes` arguments, encoded like `PRAGMA function_list`
/// encodes them, accepts `arguments` arguments
fn accepts(takes: i8, arguments: usize) -> bool {
    match takes {
        -1 => true,
        exactly @ 0.. => arguments == usize::from(exactly.unsigned_abs()),
        at_least => arguments >= usize::from((-2 - at_least).unsigned_abs()),
    }
}
