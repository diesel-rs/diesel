

CREATE TYPE auto_test_status AS ENUM ('pending', 'faulted', 'completed');
CREATE TYPE auto_test_frequency AS ENUM ('weekly', 'monthly');

CREATE TABLE AutoTestSetting (
    id SERIAL PRIMARY KEY,
    status auto_test_status NOT NULL,
    frequency auto_test_frequency NOT NULL
);
