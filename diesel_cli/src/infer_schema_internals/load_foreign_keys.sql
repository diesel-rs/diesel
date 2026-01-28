SELECT
   sch.nspname                                   AS "self_schema",
   tbl.relname                                   AS "self_table",
   ARRAY_AGG(col.attname ORDER BY u.attposition) AS "self_columns",
   f_sch.nspname                                 AS "foreign_schema",
   f_tbl.relname                                 AS "foreign_table",
   ARRAY_AGG(f_col.attname ORDER BY f_u.attposition) AS "foreign_columns"
FROM pg_constraint c
       LEFT JOIN LATERAL UNNEST(c.conkey) WITH ORDINALITY AS u(attnum, attposition) ON TRUE
       LEFT JOIN LATERAL UNNEST(c.confkey) WITH ORDINALITY AS f_u(attnum, attposition) ON f_u.attposition = u.attposition
       JOIN pg_class tbl ON tbl.oid = c.conrelid
       JOIN pg_namespace sch ON sch.oid = tbl.relnamespace
       LEFT JOIN pg_attribute col ON (col.attrelid = tbl.oid AND col.attnum = u.attnum)
       LEFT JOIN pg_class f_tbl ON f_tbl.oid = c.confrelid
       LEFT JOIN pg_namespace f_sch ON f_sch.oid = f_tbl.relnamespace
       LEFT JOIN pg_attribute f_col ON (f_col.attrelid = f_tbl.oid AND f_col.attnum = f_u.attnum)
WHERE sch.nspname = $1 and f_sch.nspname = $1 and c.contype = 'f'
GROUP BY "self_schema", "self_table", "foreign_schema", "foreign_table"
ORDER BY "self_schema", "self_table";
