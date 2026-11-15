-- Mirrors the `information_schema.columns` fields used by `get_table_data()`
SELECT a.attname AS column_name,
    COALESCE(bt.typname, t.typname) AS type_name,
    COALESCE(nbt.nspname, nt.nspname) AS type_schema,
    CASE WHEN t.typtype = 'd' THEN t.typname END AS domain_name,
    CASE WHEN t.typtype = 'd' THEN nt.nspname END AS domain_schema,
    CASE WHEN a.attnotnull OR (t.typtype = 'd' AND t.typnotnull) THEN 'NO' ELSE 'YES' END AS nullable,
    NULL::int4 AS max_length,
    col_description(a.attrelid, a.attnum) AS comment
FROM pg_attribute a
    JOIN pg_class c ON c.oid = a.attrelid
    JOIN pg_namespace nc ON nc.oid = c.relnamespace
    JOIN pg_type t ON t.oid = a.atttypid
    JOIN pg_namespace nt ON nt.oid = t.typnamespace
    LEFT JOIN pg_type bt ON t.typtype = 'd' AND bt.oid = t.typbasetype
    LEFT JOIN pg_namespace nbt ON nbt.oid = bt.typnamespace
WHERE c.relkind = 'm'
    AND c.relname = $1
    AND nc.nspname = $2
    AND a.attnum > 0
    AND NOT a.attisdropped
ORDER BY CASE WHEN $3 THEN a.attname END, a.attnum
