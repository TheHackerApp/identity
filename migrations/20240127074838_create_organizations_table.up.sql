CREATE TABLE organizations (
    id int primary key generated always as identity,
    name text not null,
    logo text,
    website text,
    owner_id int not null,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

CREATE TRIGGER set_organizations_updated_at_timestamp
    BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
