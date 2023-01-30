CREATE TABLE with_comments (
    id INTEGER PRIMARY KEY COMMENT 'column comment',
    column_without_comment INTEGER
) COMMENT 'table comment';
