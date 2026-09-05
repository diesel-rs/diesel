use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Jsonb;
use diesel::sqlite::{Sqlite, SqliteBindValue};

fn encode_jsonb(value: &serde_json::Value) {
    let bind_value = SqliteBindValue::from(None as Option<i32>);
    let mut metadata = ();
    let mut out = Output::new(bind_value, &mut metadata);
    ToSql::<Jsonb, Sqlite>::to_sql(value, &mut out).expect("JSONB serialization failed");
}

fn build_nested_array(depth: usize) -> serde_json::Value {
    let mut val = serde_json::json!(1);
    for _ in 0..depth {
        val = serde_json::Value::Array(vec![val]);
    }
    val
}

fn build_nested_object(depth: usize) -> serde_json::Value {
    let mut val = serde_json::json!(1);
    for _ in 0..depth {
        let mut map = serde_json::Map::new();
        map.insert("key".to_string(), val);
        val = serde_json::Value::Object(map);
    }
    val
}

fn build_flat_array_scalars(size: usize) -> serde_json::Value {
    serde_json::Value::Array((0..size).map(|i| serde_json::json!(i)).collect())
}

fn build_flat_array_objects(size: usize) -> serde_json::Value {
    serde_json::Value::Array(
        (0..size)
            .map(|i| {
                serde_json::json!({
                    "id": i,
                    "name": "benchmark_item"
                })
            })
            .collect(),
    )
}

fn build_flat_object_keys(size: usize) -> serde_json::Value {
    let mut map = serde_json::Map::with_capacity(size);
    for i in 0..size {
        map.insert(format!("field_{i}"), serde_json::json!(i));
    }
    serde_json::Value::Object(map)
}

use std::time::Duration;

fn bench_depth_nested_arrays(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonb_depth_nested_arrays");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_millis(700));
    group.sample_size(30);
    for &depth in &[10, 50, 100, 250, 500, 1000, 2000] {
        let val = build_nested_array(depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), &val, |b, val| {
            b.iter(|| encode_jsonb(black_box(val)));
        });
    }
    group.finish();
}

fn bench_depth_nested_objects(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonb_depth_nested_objects");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_millis(700));
    group.sample_size(30);
    for &depth in &[10, 50, 100, 250, 500, 1000] {
        let val = build_nested_object(depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), &val, |b, val| {
            b.iter(|| encode_jsonb(black_box(val)));
        });
    }
    group.finish();
}

fn bench_size_array_scalars(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonb_size_array_scalars");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_millis(700));
    group.sample_size(30);
    for &size in &[10, 100, 1000, 10000] {
        let val = build_flat_array_scalars(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &val, |b, val| {
            b.iter(|| encode_jsonb(black_box(val)));
        });
    }
    group.finish();
}

fn bench_size_array_objects(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonb_size_array_objects");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_millis(700));
    group.sample_size(30);
    for &size in &[10, 100, 1000, 5000] {
        let val = build_flat_array_objects(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &val, |b, val| {
            b.iter(|| encode_jsonb(black_box(val)));
        });
    }
    group.finish();
}

fn bench_size_object_keys(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonb_size_object_keys");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_millis(700));
    group.sample_size(30);
    for &size in &[10, 100, 1000, 5000] {
        let val = build_flat_object_keys(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &val, |b, val| {
            b.iter(|| encode_jsonb(black_box(val)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_depth_nested_arrays,
    bench_depth_nested_objects,
    bench_size_array_scalars,
    bench_size_array_objects,
    bench_size_object_keys,
);
criterion_main!(benches);
