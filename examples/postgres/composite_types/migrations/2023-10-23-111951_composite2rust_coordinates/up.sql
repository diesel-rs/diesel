-- Define the contents of the table.
CREATE TABLE coordinates (
    coord_id       SERIAL    PRIMARY KEY,
    xcoord      INTEGER   NOT NULL DEFAULT 0,
    ycoord INTEGER   NOT NULL DEFAULT 0
);

-- Fill table with some example data.
INSERT INTO coordinates (xcoord, ycoord) VALUES (1, 0);
INSERT INTO coordinates (xcoord, ycoord) VALUES (0, 1);
INSERT INTO coordinates (xcoord, ycoord) VALUES (1, 1);
INSERT INTO coordinates (xcoord, ycoord) VALUES (3, 4);

-- You can verify the function below in psql with:
-- SELECT distance_from_origin(xcoord, ycoord) FROM coordinates;
CREATE FUNCTION distance_from_origin (
    re INTEGER,
    im INTEGER)
    RETURNS FLOAT4 AS
$$
DECLARE
    dist     FLOAT4;
BEGIN
    dist = SQRT(POWER(re,2) + POWER(im,2));
    RETURN dist;
END
$$ LANGUAGE plpgsql IMMUTABLE;

-- You can verify the function below in psql with:
-- SELECT shortest_distance();
-- Which should report '(1,1)' or '(2,1)' given the above inserted data.
CREATE FUNCTION shortest_distance (
    OUT _coord_id INTEGER,
    OUT distance  FLOAT4)
    AS
$$
BEGIN
    SELECT coord_id, distance_from_origin(xcoord, ycoord)
    FROM coordinates ORDER BY distance_from_origin ASC
    INTO _coord_id, distance;
END
$$ LANGUAGE plpgsql IMMUTABLE;

-- Make a composite type in SQL
CREATE TYPE my_comp_type AS (coord_id INTEGER, distance FLOAT4);

-- You can verify the function below in psql with:
-- SELECT longest_distance();
-- Which should report '(4,5)' given the above inserted data.
CREATE FUNCTION longest_distance (
    OUT res my_comp_type)
    AS
$$
DECLARE
    _coord_id INTEGER;
    distance  FLOAT4;
BEGIN
    SELECT coord_id, distance_from_origin(xcoord, ycoord)
    FROM coordinates ORDER BY distance_from_origin DESC
    INTO _coord_id, distance;
    res := ROW(_coord_id, distance);
END
$$ LANGUAGE plpgsql IMMUTABLE;
