use crate::{
    CustomDomain, Event, Identity, Organization, Organizer, Participant, PgPool, Provider, User,
};
use async_graphql::{
    dataloader::{DataLoader, Loader, NoCache},
    SchemaBuilder,
};
use async_trait::async_trait;
use std::collections::HashMap;

macro_rules! declare_loader {
    ($name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty )) => {
        declare_loader!($name<$impl_name> for $model => $key($key_type) using load providing $model);
    };
    ($name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty ) using $method:ident) => {
        declare_loader!($name<$impl_name> for $model => $key($key_type) using $method providing $model);
    };
    ($name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty ) providing $result:ty) => {
        declare_loader!($name<$impl_name> for $model => $key($key_type) using load providing $result);
    };
    ($name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty ) using $method:ident providing $result:ty) => {
        #[doc = concat!("Efficiently load [`", stringify!($model), "`]s in GraphQL queries/mutations")]
        pub type $name = DataLoader<$impl_name, NoCache>;

        #[doc = concat!("The dataloader implementation for [`", stringify!($model), "`]s")]
        pub struct $impl_name(PgPool);

        impl $impl_name {
            #[doc = concat!("Create a new dataloader for [`", stringify!($model), "`]s")]
            #[inline(always)]
            fn new(db: &PgPool) -> $name {
                DataLoader::new($impl_name(db.clone()), tokio::task::spawn)
            }
        }

        #[async_trait]
        impl Loader<$key_type> for $impl_name {
            type Value = $result;
            type Error = $crate::Error;

            async fn load(
                &self,
                keys: &[$key_type],
            ) -> Result<HashMap<$key_type, Self::Value>, Self::Error> {
                <$model>::$method(keys, &self.0).await
            }
        }
    };
}

declare_loader!(CustomDomainLoader<CustomDomainLoaderImpl> for CustomDomain => event(String));
declare_loader!(EventLoader<EventLoaderImpl> for Event => slug(String));
declare_loader!(EventsForOrganizationLoader<EventsForOrganizationLoaderImpl> for Event => organization_id(i32) using load_for_organizations providing Vec<Event>);
declare_loader!(EventsForUserLoader<EventsForUserLoaderImpl> for Participant => user_id(i32) using load_for_user providing Vec<Participant>);
declare_loader!(IdentitiesForUserLoader<IdentitiesForUserLoaderImpl> for Identity => user_id(i32) using load_for_user providing Vec<Identity>);
declare_loader!(OrganizationLoader<OrganizationLoaderImpl> for Organization => id(i32));
declare_loader!(OrganizationsForUserLoader<OrganizationsForUserLoaderImpl> for Organizer => user_id(i32) using load_for_user providing Vec<Organizer>);
declare_loader!(ProviderLoader<ProviderLoaderImpl> for Provider => slug(String));
declare_loader!(UserLoader<UserLoaderImpl> for User => id(i32));
declare_loader!(UserByPrimaryEmailLoader<UserByPrimaryEmailLoaderImpl> for User => primary_email(String) using load_by_primary_email);
declare_loader!(UsersForEventLoader<UsersForEventLoaderImpl> for Participant => event(String) using load_for_event providing Vec<Participant>);
declare_loader!(UsersForOrganizationLoader<UsersForOrganizationLoaderImpl> for Organizer => organization_id(i32) using load_for_organization providing Vec<Organizer>);

/// Registers the defined dataloaders
pub trait RegisterDataLoaders {
    fn register_dataloaders(self, db: &PgPool) -> Self;
}

impl<Q, M, S> RegisterDataLoaders for SchemaBuilder<Q, M, S> {
    fn register_dataloaders(self, db: &PgPool) -> Self {
        self.data(CustomDomainLoaderImpl::new(db))
            .data(EventLoaderImpl::new(db))
            .data(EventsForOrganizationLoaderImpl::new(db))
            .data(EventsForUserLoaderImpl::new(db))
            .data(IdentitiesForUserLoaderImpl::new(db))
            .data(OrganizationLoaderImpl::new(db))
            .data(OrganizationsForUserLoaderImpl::new(db))
            .data(ProviderLoaderImpl::new(db))
            .data(UserLoaderImpl::new(db))
            .data(UserByPrimaryEmailLoaderImpl::new(db))
            .data(UsersForEventLoaderImpl::new(db))
            .data(UsersForOrganizationLoaderImpl::new(db))
    }
}
