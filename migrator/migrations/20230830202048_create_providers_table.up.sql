CREATE TABLE providers (
    slug text primary key,
    enabled bool not null default false,
    name text not null,
    icon text not null,
    config json not null,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

CREATE FUNCTION set_updated_at_timestamp()
RETURNS TRIGGER AS $$
    BEGIN
        new.updated_at = now();
        RETURN new;
    END;
$$ LANGUAGE 'plpgsql';

CREATE TRIGGER set_providers_updated_at_timestamp
    BEFORE UPDATE ON providers
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
