CREATE TABLE with_comments (
    id INTEGER PRIMARY KEY,
    column_without_comment INTEGER
);
COMMENT ON TABLE with_comments IS 'table comment';
COMMENT ON COLUMN with_comments.id IS 'column comment';
