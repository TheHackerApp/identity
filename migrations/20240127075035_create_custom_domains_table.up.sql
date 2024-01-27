CREATE TABLE custom_domains (
    event text primary key references events (slug),
    name text not null,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

CREATE UNIQUE INDEX ON custom_domains (name);

CREATE TRIGGER set_custom_domains_updated_at_timestamp
    BEFORE UPDATE ON custom_domains
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
