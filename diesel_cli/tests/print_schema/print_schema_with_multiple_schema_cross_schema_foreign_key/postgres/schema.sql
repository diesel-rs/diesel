CREATE SCHEMA people;
CREATE SCHEMA game;

CREATE TABLE people.api_token (
    id SERIAL PRIMARY KEY
);

CREATE TABLE game.game_session (
    id SERIAL PRIMARY KEY,
    api_token_id INTEGER NOT NULL REFERENCES people.api_token(id)
);
