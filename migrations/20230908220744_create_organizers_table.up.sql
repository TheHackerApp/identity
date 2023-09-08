CREATE TABLE organizers (
    organization_id int not null,
    user_id int not null references users (id),
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now(),
    primary key (organization_id, user_id)
);

CREATE TRIGGER set_organizers_updated_at_timestamp
    BEFORE UPDATE ON organizers
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
