mod diesel_benches;
#[cfg(all(feature = "mysql", feature = "rust_mysql"))]
mod mysql_benches;
#[cfg(all(feature = "postgres", feature = "rust_postgres"))]
mod postgres_benches;
#[cfg(feature = "quaint")]
mod quaint_benches;
#[cfg(all(feature = "rusqlite", feature = "sqlite"))]
mod rusqlite_benches;
#[cfg(feature = "rustorm")]
mod rust_orm_benches;
#[cfg(feature = "sqlx")]
mod sqlx_benches;

use criterion::{BenchmarkId, Criterion};

#[cfg(any(feature = "sqlite", feature = "rustorm"))]
const SQLITE_MIGRATION_SQL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name VARCHAR NOT NULL,
  hair_color VARCHAR
);
",
    "CREATE TABLE IF NOT EXISTS posts (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  user_id INTEGER NOT NULL,
  title VARCHAR NOT NULL,
  body TEXT
);
",
    "CREATE TABLE IF NOT EXISTS comments (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  post_id INTEGER NOT NULL,
  text TEXT NOT NULL
);
",
];

const TRIVIAL_QUERY_SIZE: &[usize] = &[1, 10, 100, 1_000, 10_000];
const MEDIUM_COMPLEX_SIZE: &[usize] = &[1, 10, 100, 1_000, 10_000];
const INSERT_SIZE: &[usize] = &[1, 10, 25, 50, 100];

fn bench_trivial_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_trivial_query");

    for size in TRIVIAL_QUERY_SIZE {
        group.bench_with_input(BenchmarkId::new("diesel", size), size, |b, i| {
            crate::diesel_benches::bench_trivial_query(b, *i);
        });

        group.bench_with_input(BenchmarkId::new("diesel_boxed", size), size, |b, i| {
            crate::diesel_benches::bench_trivial_query_boxed(b, *i);
        });

        group.bench_with_input(
            BenchmarkId::new("diesel_queryable_by_name", size),
            size,
            |b, i| crate::diesel_benches::bench_trivial_query_raw(b, *i),
        );

        #[cfg(feature = "rustorm")]
        group.bench_with_input(BenchmarkId::new("rustorm", size), size, |b, i| {
            crate::rust_orm_benches::bench_trivial_query(b, *i);
        });

        #[cfg(feature = "quaint")]
        group.bench_with_input(BenchmarkId::new("quaint", size), size, |b, i| {
            crate::quaint_benches::bench_trivial_query(b, *i);
        });

        #[cfg(feature = "sqlx")]
        group.bench_with_input(
            BenchmarkId::new("sqlx_query_as_macro", size),
            size,
            |b, i| {
                crate::sqlx_benches::bench_trivial_query_query_as_macro(b, *i);
            },
        );
        #[cfg(feature = "sqlx")]
        group.bench_with_input(
            BenchmarkId::new("sqlx_query_from_row", size),
            size,
            |b, i| {
                crate::sqlx_benches::bench_trivial_query_from_row(b, *i);
            },
        );

        #[cfg(all(feature = "sqlite", feature = "rusqlite"))]
        group.bench_with_input(BenchmarkId::new("rusqlite_by_id", size), size, |b, i| {
            crate::rusqlite_benches::bench_trivial_query_by_id(b, *i);
        });

        #[cfg(all(feature = "sqlite", feature = "rusqlite"))]
        group.bench_with_input(BenchmarkId::new("rusqlite_by_name", size), size, |b, i| {
            crate::rusqlite_benches::bench_trivial_query_by_name(b, *i);
        });

        #[cfg(all(feature = "postgres", feature = "rust_postgres"))]
        group.bench_with_input(BenchmarkId::new("postgres_by_id", size), size, |b, i| {
            crate::postgres_benches::bench_trivial_query_by_id(b, *i);
        });

        #[cfg(all(feature = "postgres", feature = "rust_postgres"))]
        group.bench_with_input(BenchmarkId::new("postgres_by_name", size), size, |b, i| {
            crate::postgres_benches::bench_trivial_query_by_name(b, *i);
        });

        #[cfg(all(feature = "mysql", feature = "rust_mysql"))]
        group.bench_with_input(BenchmarkId::new("mysql_by_id", size), size, |b, i| {
            crate::mysql_benches::bench_trivial_query_by_id(b, *i);
        });

        #[cfg(all(feature = "mysql", feature = "rust_mysql"))]
        group.bench_with_input(BenchmarkId::new("mysql_by_name", size), size, |b, i| {
            crate::mysql_benches::bench_trivial_query_by_name(b, *i);
        });
    }

    group.finish();
}

fn bench_medium_complex_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_medium_complex_query");

    for size in MEDIUM_COMPLEX_SIZE {
        group.bench_with_input(BenchmarkId::new("diesel", size), size, |b, i| {
            crate::diesel_benches::bench_medium_complex_query(b, *i)
        });
        group.bench_with_input(BenchmarkId::new("diesel_boxed", size), size, |b, i| {
            crate::diesel_benches::bench_medium_complex_query_boxed(b, *i)
        });
        group.bench_with_input(
            BenchmarkId::new("diesel_queryable_by_name", size),
            size,
            |b, i| crate::diesel_benches::bench_medium_complex_query_queryable_by_name(b, *i),
        );

        #[cfg(feature = "quaint")]
        group.bench_with_input(BenchmarkId::new("quaint", size), size, |b, i| {
            crate::quaint_benches::bench_medium_complex_query(b, *i);
        });

        #[cfg(feature = "sqlx")]
        group.bench_with_input(
            BenchmarkId::new("sqlx_query_as_macro", size),
            size,
            |b, i| {
                crate::sqlx_benches::bench_medium_complex_query_query_as_macro(b, *i);
            },
        );

        #[cfg(feature = "sqlx")]
        group.bench_with_input(
            BenchmarkId::new("sqlx_query_from_row", size),
            size,
            |b, i| crate::sqlx_benches::bench_medium_complex_query_from_row(b, *i),
        );

        #[cfg(feature = "rustorm")]
        group.bench_with_input(BenchmarkId::new("rustorm", size), size, |b, i| {
            crate::rust_orm_benches::bench_medium_complex_query(b, *i)
        });

        #[cfg(all(feature = "sqlite", feature = "rusqlite"))]
        group.bench_with_input(BenchmarkId::new("rusqlite_by_id", size), size, |b, i| {
            crate::rusqlite_benches::bench_medium_complex_query_by_id(b, *i);
        });

        #[cfg(all(feature = "sqlite", feature = "rusqlite"))]
        group.bench_with_input(BenchmarkId::new("rusqlite_by_name", size), size, |b, i| {
            crate::rusqlite_benches::bench_medium_complex_query_by_name(b, *i);
        });

        #[cfg(all(feature = "postgres", feature = "rust_postgres"))]
        group.bench_with_input(BenchmarkId::new("postgres_by_id", size), size, |b, i| {
            crate::postgres_benches::bench_medium_complex_query_by_id(b, *i);
        });

        #[cfg(all(feature = "postgres", feature = "rust_postgres"))]
        group.bench_with_input(BenchmarkId::new("postgres_by_name", size), size, |b, i| {
            crate::postgres_benches::bench_medium_complex_query_by_name(b, *i);
        });

        #[cfg(all(feature = "mysql", feature = "rust_mysql"))]
        group.bench_with_input(BenchmarkId::new("mysql_by_id", size), size, |b, i| {
            crate::mysql_benches::bench_medium_complex_query_by_id(b, *i);
        });

        #[cfg(all(feature = "mysql", feature = "rust_mysql"))]
        group.bench_with_input(BenchmarkId::new("mysql_by_name", size), size, |b, i| {
            crate::mysql_benches::bench_medium_complex_query_by_name(b, *i);
        });
    }

    group.finish();
}

fn bench_loading_associations_sequentially(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_loading_associations_sequentially");

    group.bench_function("diesel/bench_loading_associations_sequentially", |b| {
        crate::diesel_benches::loading_associations_sequentially(b)
    });

    #[cfg(all(feature = "postgres", feature = "rust_postgres"))]
    group.bench_function("postgres", |b| {
        crate::postgres_benches::loading_associations_sequentially(b)
    });

    #[cfg(all(feature = "sqlite", feature = "rusqlite"))]
    group.bench_function("rusqlite", |b| {
        crate::rusqlite_benches::loading_associations_sequentially(b)
    });

    #[cfg(feature = "sqlx")]
    group.bench_function("sqlx", |b| {
        crate::sqlx_benches::loading_associations_sequentially(b)
    });

    #[cfg(feature = "rustorm")]
    group.bench_function("rustorm", |b| {
        crate::rust_orm_benches::loading_associations_sequentially(b)
    });

    #[cfg(feature = "quaint")]
    group.bench_function("quaint", |b| {
        crate::quaint_benches::loading_associations_sequentially(b)
    });

    #[cfg(all(feature = "mysql", feature = "rust_mysql"))]
    group.bench_function("mysql", |b| {
        crate::mysql_benches::loading_associations_sequentially(b)
    });

    group.finish();
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_insert");

    for size in INSERT_SIZE {
        group.bench_with_input(BenchmarkId::new("diesel", size), size, |b, i| {
            crate::diesel_benches::bench_insert(b, *i);
        });

        #[cfg(feature = "sqlx")]
        group.bench_with_input(BenchmarkId::new("sqlx", size), size, |b, i| {
            crate::sqlx_benches::bench_insert(b, *i);
        });

        #[cfg(feature = "quaint")]
        group.bench_with_input(BenchmarkId::new("quaint", size), size, |b, i| {
            crate::quaint_benches::bench_insert(b, *i);
        });

        #[cfg(feature = "rustorm")]
        group.bench_with_input(BenchmarkId::new("rustorm", size), size, |b, i| {
            crate::rust_orm_benches::bench_insert(b, *i);
        });

        #[cfg(all(feature = "postgres", feature = "rust_postgres"))]
        group.bench_with_input(BenchmarkId::new("postgres", size), size, |b, i| {
            crate::postgres_benches::bench_insert(b, *i);
        });

        #[cfg(all(feature = "sqlite", feature = "rusqlite"))]
        group.bench_with_input(BenchmarkId::new("rusqlite", size), size, |b, i| {
            crate::rusqlite_benches::bench_insert(b, *i);
        });

        #[cfg(all(feature = "mysql", feature = "rust_mysql"))]
        group.bench_with_input(BenchmarkId::new("mysql", size), size, |b, i| {
            crate::mysql_benches::bench_insert(b, *i);
        });
    }

    group.finish();
}

criterion::criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_trivial_query, bench_medium_complex_query, bench_loading_associations_sequentially, bench_insert
);

criterion::criterion_main!(benches);
