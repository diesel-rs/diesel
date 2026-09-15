# diesel-fuzz

Fuzz harnesses for diesel's deserialization code, reaching diesel only through its public API.

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

- Only `sqlite_jsonb_decode` has a checked-in corpus; the other targets take `Arbitrary` input.
- `sqlite_jsonb_roundtrip` builds its value from entropy, not parsed `JSON`, so a writer bug cannot hide behind matched `serde_json` rounding.
- `-max_len=4096` avoids the drop-time stack overflow in `serde_json::Value`.
- Minimize a crash with `cargo fuzz tmin`, then pin it as a test in the owning diesel module.
