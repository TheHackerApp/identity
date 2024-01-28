CREATE TYPE organizer_role AS ENUM ('director', 'manager', 'organizer');

ALTER TABLE organizers ADD COLUMN role organizer_role NOT NULL DEFAULT 'organizer';
