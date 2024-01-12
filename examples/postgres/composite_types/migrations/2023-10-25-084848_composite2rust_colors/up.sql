-- Define the contents of the table.
CREATE TABLE colors (
    color_id    SERIAL    PRIMARY KEY,
    red         INTEGER   NOT NULL DEFAULT 0,
    green       INTEGER   NOT NULL DEFAULT 0,
    blue        INTEGER   NOT NULL DEFAULT 0,
    color_name  TEXT
);

-- Fill table with some example data from /etc/X11/rgb.txt
INSERT INTO colors (red, green,blue, color_name) VALUES (139,  71, 137, 'orchid4');
INSERT INTO colors (red, green,blue, color_name) VALUES (139, 123, 139, 'thistle4');
INSERT INTO colors (red, green,blue, color_name) VALUES (205,  41, 144, 'maroon3');
INSERT INTO colors (red, green,blue, color_name) VALUES (205,  79,  57, 'tomato3');
INSERT INTO colors (red, green,blue, color_name) VALUES (238, 130,  98, 'salmon2');
INSERT INTO colors (red, green,blue, color_name) VALUES (238, 216, 174, 'wheat2');
INSERT INTO colors (red, green,blue, color_name) VALUES (205, 198, 115, 'khaki3');
INSERT INTO colors (red, green,blue, color_name) VALUES (118, 238, 198, 'aquamarine');

-- Make a composite type in SQL
CREATE TYPE gray_type AS (intensity FLOAT4, suggestion TEXT);

-- Converts an color to a grey value and suggest a name.
CREATE FUNCTION color2grey(
    red    INTEGER,
    green  INTEGER,
    blue   INTEGER
) RETURNS gray_type AS
$$
DECLARE
    intensity   FLOAT4;
    percentage  FLOAT4;
    suggestion  TEXT;
    rec         gray_type;
BEGIN
    intensity  := (red::FLOAT4 + green::FLOAT4 + blue::FLOAT4 ) / 3;
    percentage := (100 * intensity / 256)::INTEGER;
    IF (percentage > 99) THEN
        percentage := 99;
    ELSE
        IF (percentage < 0) THEN
            percentage := 0;
        END IF;
    END IF;
    suggestion := 'grey' || percentage::TEXT;
    rec := ROW(intensity,suggestion);
    RETURN rec;
END
$$ LANGUAGE plpgsql IMMUTABLE;

-- Converts an color to a gray value and suggest a name (same but 'e' -> 'a').
CREATE FUNCTION color2gray(
    red    INTEGER,
    green  INTEGER,
    blue   INTEGER
) RETURNS gray_type AS
$$
DECLARE
    intensity   FLOAT4;
    percentage  FLOAT4;
    suggestion  TEXT;
    rec         gray_type;
BEGIN
    intensity  := (red::FLOAT4 + green::FLOAT4 + blue::FLOAT4 ) / 3;
    percentage := (100 * intensity / 256)::INTEGER;
    IF (percentage > 99) THEN
        percentage := 99;
    ELSE
        IF (percentage < 0) THEN
            percentage := 0;
        END IF;
    END IF;
    suggestion := 'gray' || percentage::TEXT;
    rec := ROW(intensity,suggestion);
    RETURN rec;
END
$$ LANGUAGE plpgsql IMMUTABLE;
