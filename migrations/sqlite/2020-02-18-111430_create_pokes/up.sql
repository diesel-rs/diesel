create table pokes (
    user_id INTEGER PRIMARY KEY NOT NULL REFERENCES users(id),
    poke_count INTEGER NOT NULL,
    CONSTRAINT pokes_poke_count_check CHECK (poke_count > 0)
);
