CREATE SCHEMA v2;

CREATE TYPE v2.auto_test_status AS ENUM ('pending', 'faulted', 'completed');
CREATE TYPE v2.auto_test_frequency AS ENUM ('weekly', 'monthly');

CREATE TABLE v2.AutoTestSetting (
    hardware_id Text PRIMARY KEY,
    enabled boolean NOT NULL,
    email_notifications boolean NOT NULL,
    sms_notifications boolean NOT NULL,
    push_notifications boolean NOT NULL,
    start_hour text NOT NULL,
    status v2.auto_test_status NOT NULL,
    frequency v2.auto_test_frequency NOT NULL,
    day integer NOT NULL,
    last_test timestamptz NULL,
    next_test timestamptz NULL
);
