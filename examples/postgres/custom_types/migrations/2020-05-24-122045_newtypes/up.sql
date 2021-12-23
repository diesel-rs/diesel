CREATE TYPE Language AS ENUM (
    'en', 'ru', 'de'
);

CREATE TABLE translations (
    word_id INTEGER NOT NULL,
    translation_id INTEGER NOT NULL,
    language Language NOT NULL,

    PRIMARY KEY (word_id, translation_id)
)
