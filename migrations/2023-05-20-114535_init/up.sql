-- Your SQL goes here
CREATE TABLE visits (
    id SERIAL PRIMARY KEY,
    visitor TEXT NOT NULL,
    path TEXT NOT NULL,
    instance TIMESTAMP NOT NULL
);
