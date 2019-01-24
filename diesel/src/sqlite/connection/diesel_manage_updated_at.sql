CREATE TRIGGER __diesel_manage_updated_at_{table_name}
AFTER UPDATE ON {table_name}
FOR EACH ROW WHEN
  old.updated_at IS NULL AND
  new.updated_at IS NULL OR
  old.updated_at == new.updated_at
BEGIN
  UPDATE {table_name}
  SET updated_at = CURRENT_TIMESTAMP
  WHERE ROWID = new.ROWID;
END
