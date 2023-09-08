CREATE TABLE participants (
    event text not null,
    user_id int not null references users (id),
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now(),
    primary key (event, user_id)
);

CREATE TRIGGER set_participants_updated_at_timestamp
    BEFORE UPDATE ON participants
    FOR EACH ROW EXECUTE PROCEDURE set_updated_at_timestamp();
