use async_graphql::{MergedObject, SimpleObject};

mod identity;
mod providers;
mod user;
mod validators;

use identity::IdentityMutation;
use providers::ProviderMutation;
use user::UserMutation;

/// The various GraphQL mutations
///
/// To improve readability, the mutation implementations are split into different files, but all
/// attached to this one struct.
#[derive(Default, MergedObject)]
pub struct Mutation(IdentityMutation, ProviderMutation, UserMutation);

/// Represents and error in the input of a mutation
#[derive(Debug, SimpleObject)]
pub struct UserError {
    /// The path to the input field that caused the error
    field: &'static [&'static str],
    /// The error message
    message: String,
}

impl UserError {
    /// Create a new user error
    pub fn new(field: &'static [&'static str], message: impl ToString) -> Self {
        let message = message.to_string();
        Self { field, message }
    }
}

/// Create mutation results with user errors
macro_rules! results {
    (
        $(
            $( #[$outer:meta] )*
            $name:ident {
                $( #[$inner:meta] )*
                $field:ident : $type:ty $(,)?
            }
        )*
    ) => {
        $(
            $( #[$outer] )*
            #[derive(Debug, async_graphql::SimpleObject)]
            struct $name {
                $( #[$inner] )*
                $field: Option<$type>,
                /// Errors that may have occurred while processing the action
                user_errors: Vec<$crate::mutation::UserError>
            }

            impl From<$type> for $name {
                fn from(value: $type) -> Self {
                    Self {
                        $field: Some(value),
                        user_errors: Vec::with_capacity(0),
                    }
                }
            }

            impl From<$crate::mutation::UserError> for $name {
                fn from(user_error: $crate::mutation::UserError) -> Self {
                    Self {
                        $field: None,
                        user_errors: vec![user_error],
                    }
                }
            }

            impl From<Vec<$crate::mutation::UserError>> for $name {
                fn from(user_errors: Vec<$crate::mutation::UserError>) -> Self {
                    Self {
                        $field: None,
                        user_errors,
                    }
                }
            }
        )*
    };
}

pub(crate) use results;
