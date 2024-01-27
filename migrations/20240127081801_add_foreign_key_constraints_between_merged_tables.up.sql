ALTER TABLE participants
ADD CONSTRAINT participants_event_fkey FOREIGN KEY (event) REFERENCES events (slug);

ALTER TABLE organizers
ADD CONSTRAINT organizers_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations (id);

ALTER TABLE organizations
ADD CONSTRAINT organizations_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES users (id);
