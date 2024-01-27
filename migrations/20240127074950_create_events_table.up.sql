CREATE TABLE events (
    slug text primary key,
    name text not null,
    organization_id int not null references organizations (id),
    expires_on timestamp with time zone not null default now() + interval '6 months',
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

CREATE INDEX ON events (organization_id);

CREATE TRIGGER set_events_updated_at_timestamp
    BEFORE UPDATE ON events
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
