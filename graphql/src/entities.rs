use async_graphql::InputObject;

/// A minimal event model, for use in entity keys
#[derive(Debug, InputObject)]
pub(crate) struct Event {
    pub slug: String,
}

/// A minimal organization model, for use in entity keys
#[derive(Debug, InputObject)]
pub(crate) struct Organization {
    pub id: i32,
}

/// A minimal user model, for use in entity keys
#[derive(Debug, InputObject)]
pub(crate) struct User {
    pub id: i32,
}
