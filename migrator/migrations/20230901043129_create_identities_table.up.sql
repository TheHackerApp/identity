CREATE TABLE identities (
    provider text not null references providers (slug),
    user_id int not null references users (id),
    remote_id text not null,
    email text not null,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now(),
    primary key (provider, user_id)
);

CREATE INDEX ON identities (provider, remote_id);

CREATE TRIGGER set_identities_updated_at_timestamp
    BEFORE UPDATE ON identities
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
