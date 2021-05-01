-- Your SQL goes here
CREATE TABLE talks (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    talk_type TINYINT NOT NULL,
    description TEXT NOT NULL,
    is_visible BOOLEAN DEFAULT TRUE NOT NULL
);

CREATE INDEX visible_talks ON talks (is_visible);
