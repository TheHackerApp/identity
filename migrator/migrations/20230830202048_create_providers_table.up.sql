CREATE TABLE providers (
    slug text primary key,
    enabled bool not null default false,
    name text not null,
    icon text not null,
    config json not null
);
