CREATE SCHEMA people;
CREATE SCHEMA game;

CREATE TABLE people."1st_user" (
    id SERIAL PRIMARY KEY
);

CREATE TABLE people."1st_api_token" (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES people."1st_user"(id)
);

CREATE TABLE game."2nd_game_session" (
    id SERIAL PRIMARY KEY,
    api_token_id INTEGER NOT NULL REFERENCES people."1st_api_token"(id)
);
