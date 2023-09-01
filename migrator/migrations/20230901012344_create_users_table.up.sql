CREATE TABLE users (
    id int primary key generated always as identity,
    given_name text not null,
    family_name text not null,
    primary_email text not null unique,
    is_admin boolean not null default false,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

CREATE INDEX ON users (primary_email);

CREATE TRIGGER set_users_updated_at_timestamp
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
