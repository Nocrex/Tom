-- Your SQL goes here
CREATE TABLE reporters (
    id bigint PRIMARY KEY,
    steamid bigint
);

CREATE TABLE reports (
    id serial PRIMARY KEY,
    reporter bigint NOT NULL REFERENCES reporters,
    time timestamp NOT NULL,
    points smallint NOT NULL,
    threadurl text NOT NULL,
    message text NOT NULL
);

CREATE TABLE playerreports (
    report int NOT NULL REFERENCES reports ON DELETE CASCADE,
    steamid bigint NOT NULL,
    last_seen timestamp NOT NULL,
    attribute smallint NOT NULL,
    verified boolean NOT NULL,
    PRIMARY KEY (report, steamid)
);