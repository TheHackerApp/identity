use crate::{Identity, PgPool, Provider, User};
use async_graphql::dataloader::{DataLoader, Loader, NoCache};
use async_trait::async_trait;
use std::collections::HashMap;

macro_rules! declare_loader {
    ($creator:ident: $name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty )) => {
        declare_loader!($creator: $name<$impl_name> for $model => $key($key_type) using load providing $model);
    };
    ($creator:ident:  $name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty ) using $method:ident) => {
        declare_loader!($creator: $name<$impl_name> for $model => $key($key_type) using $method providing $model);
    };
    ($creator:ident: $name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty ) providing $result:ty) => {
        declare_loader!($creator: $name<$impl_name> for $model => $key($key_type) using load providing $result);
    };
    ($creator:ident: $name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty ) using $method:ident providing $result:ty) => {
        #[doc = concat!("Efficiently load [`", stringify!($model), "`]s in GraphQL queries/mutations")]
        pub type $name = DataLoader<$impl_name, NoCache>;

        #[doc = concat!("Create a new dataloader for [`", stringify!($model), "`]s")]
        pub fn $creator(db: &PgPool) -> $name {
            DataLoader::new($impl_name(db.clone()), tokio::task::spawn)
        }

        #[doc = concat!("The dataloader implementation for [`", stringify!($model), "`]s")]
        pub struct $impl_name(PgPool);

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

declare_loader!(identity_for_user: IdentityForUserLoader<IdentityForUserLoaderImpl> for Identity => user_id(i32) using load_for_user providing Vec<Identity>);
declare_loader!(provider: ProviderLoader<ProviderLoaderImpl> for Provider => slug(String));
declare_loader!(user: UserLoader<UserLoaderImpl> for User => id(i32));
declare_loader!(user_by_primary_email: UserByPrimaryEmailLoader<UserByPrimaryEmailLoaderImpl> for User => primary_email(String) using load_by_primary_email);
