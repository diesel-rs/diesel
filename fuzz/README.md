# diesel-fuzz

Fuzz harnesses for diesel's database input and codec boundaries, reaching diesel only through its public API.

```
cargo +nightly fuzz run --fuzz-dir fuzz <target>   # from the repository root
```

| Target | Property |
|---|---|
| `pg_from_sql` | 48 postgres decoders never panic |
| `mysql_from_sql` | 29 mysql decoders never panic, under every wire type |
| `sqlite_from_sql` | 26 sqlite decoders never panic, in every storage class |
| `sqlite_jsonb_decode` | decoding a blob never panics |
| `sqlite_jsonb_roundtrip` | diesel reads back what it wrote as jsonb and json text, and sqlite calls both valid |
| `sqlite_blob_state` | incremental blob reads, seeks, and close or drop agree with a byte-slice model |
| `infer_view` | `diesel_infer_query` infers one column per column of the view, and a column it infers NOT NULL is never NULL in the rows SQLite returns |

- Only `sqlite_jsonb_decode` and `infer_view` have checked-in corpora; the other targets take `Arbitrary` input.
- `infer_view` treats input with an even first byte as raw SQL over fixed `users`, `posts`, and `comments` tables, and generates the tables and the view from any other input. Like print-schema, it infers the definition SQLite stored for a view it accepted, which also keeps garbage that sqlparser takes exponential time to reject away from the parser. It denies `random()` and `randomblob()`, so a verdict replays unless the view reads the current time, like `date('now')` does.
- `sqlite_jsonb_roundtrip` builds its value from entropy, not parsed `JSON`, so a writer bug cannot hide behind matched `serde_json` rounding.
- `-max_len=4096` avoids the drop-time stack overflow in `serde_json::Value`.
- Minimize a crash with `cargo fuzz tmin`, then pin it as a test in the owning diesel module.
