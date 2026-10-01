//! Criterion benchmarks for the two-pass SQLite JSONB encoding introduced in
//! PR #5208.
//!
//! These benchmarks measure `<serde_json::Value as ToSql<Jsonb, Sqlite>>::to_sql`,
//! which is the sole public entry-point for the two-pass iterative encoder.
//!
//! Run with:
//!   cargo bench --bench sqlite_jsonb_encode --features "sqlite serde_json"
//!
//! The benchmarks deliberately use a NULL initial `SqliteBindValue` so that
//! the `Output` carries no pre-allocated buffer — all allocation comes from
//! the encoder itself, giving a clean measurement of encoding cost.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Jsonb;
use diesel::sqlite::{Sqlite, SqliteBindValue};
use serde_json::{Value, json};

/// Encode `value` to JSONB using Diesel's `ToSql` implementation.
///
/// Panics on serialization error (should never happen with well-formed JSON).
fn encode_jsonb(value: &Value) -> SqliteBindValue<'static> {
    let null_buf: SqliteBindValue<'static> = SqliteBindValue::from(None::<Vec<u8>>);
    let mut out = Output::test(null_buf);
    <Value as ToSql<Jsonb, Sqlite>>::to_sql(value, &mut out)
        .expect("JSONB serialization failed");
    // `out` owns the result; we move it out via `Output::into_bind_value`.
    // The encoded bytes are now stored inside the `SqliteBindValue`.
    // We return it so criterion can black-box the result and prevent
    // the compiler from optimising the call away.
    out.into_inner()
}

fn bench_jsonb_encode(c: &mut Criterion) {
    // ── Fixtures ──────────────────────────────────────────────────────────────

    let scalar_int: Value = json!(42);
    let scalar_str: Value = json!("hello world");
    let scalar_bool: Value = json!(true);
    let scalar_null: Value = json!(null);
    let scalar_textj: Value = json!("line1\nline2\ttab");  // needs TEXTJ encoding

    let flat_obj: Value = json!({
        "name": "Alice",
        "age": 30,
        "active": true,
        "score": 9.75,
    });

    let flat_arr_10: Value = {
        let vals: Vec<Value> = (0..10i64).map(|i| json!(i)).collect();
        Value::Array(vals)
    };

    let flat_arr_100: Value = {
        let vals: Vec<Value> = (0..100i64).map(|i| json!(i)).collect();
        Value::Array(vals)
    };

    // Deeply nested: 10 levels
    let mut nested: Value = json!({ "leaf": 1 });
    for i in 0..10usize {
        nested = json!({ format!("level_{i}"): nested });
    }

    // Mixed nested object with arrays of objects
    let mixed: Value = json!({
        "users": [
            { "id": 1, "name": "Alice", "tags": ["admin", "user"] },
            { "id": 2, "name": "Bob",   "tags": ["user"] },
            { "id": 3, "name": "Carol", "tags": ["user", "moderator"] },
        ],
        "meta": {
            "total": 3,
            "page": 1,
            "per_page": 10,
        },
        "ok": true,
    });

    // Wide object: 50 string-keyed integer values
    let wide_obj: Value = {
        let mut m = serde_json::Map::new();
        for i in 0..50usize {
            m.insert(format!("field_{i:02}"), json!(i as i64));
        }
        Value::Object(m)
    };

    // ── Scalar benchmarks ─────────────────────────────────────────────────────
    {
        let mut g = c.benchmark_group("jsonb_encode/scalar");
        g.bench_function("int",    |b| b.iter(|| encode_jsonb(black_box(&scalar_int))));
        g.bench_function("str",    |b| b.iter(|| encode_jsonb(black_box(&scalar_str))));
        g.bench_function("textj",  |b| b.iter(|| encode_jsonb(black_box(&scalar_textj))));
        g.bench_function("bool",   |b| b.iter(|| encode_jsonb(black_box(&scalar_bool))));
        g.bench_function("null",   |b| b.iter(|| encode_jsonb(black_box(&scalar_null))));
        g.finish();
    }

    // ── Object benchmarks ─────────────────────────────────────────────────────
    {
        let mut g = c.benchmark_group("jsonb_encode/object");
        g.bench_function("flat_4keys",      |b| b.iter(|| encode_jsonb(black_box(&flat_obj))));
        g.bench_function("wide_50keys",     |b| b.iter(|| encode_jsonb(black_box(&wide_obj))));
        g.bench_function("nested_10levels", |b| b.iter(|| encode_jsonb(black_box(&nested))));
        g.bench_function("mixed_users",     |b| b.iter(|| encode_jsonb(black_box(&mixed))));
        g.finish();
    }

    // ── Array benchmarks ──────────────────────────────────────────────────────
    {
        let mut g = c.benchmark_group("jsonb_encode/array");
        g.bench_with_input(BenchmarkId::new("flat_int", 10),  &flat_arr_10,  |b, v| b.iter(|| encode_jsonb(black_box(v))));
        g.bench_with_input(BenchmarkId::new("flat_int", 100), &flat_arr_100, |b, v| b.iter(|| encode_jsonb(black_box(v))));
        g.finish();
    }
}

criterion_group!(benches, bench_jsonb_encode);
criterion_main!(benches);
