CREATE DOMAIN posinteger AS integer CHECK (VALUE > 0);

CREATE DOMAIN neginteger AS integer CHECK (VALUE < 0);

CREATE DOMAIN longtext AS text CHECK (LENGTH (VALUE) > 10);

CREATE DOMAIN shorttext AS text CHECK (LENGTH (VALUE) < 10);

CREATE TABLE mytable (
    id int PRIMARY KEY,
    a posinteger NOT NULL,
    b neginteger NOT NULL,
    c shorttext NOT NULL,
    d longtext NOT NULL
);
