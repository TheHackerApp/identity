use async_graphql::dataloader::{DataLoader, Loader, NoCache};
use async_trait::async_trait;
use database::{PgPool, Provider, User};
use futures::stream::TryStreamExt;
use std::{collections::HashMap, sync::Arc};

macro_rules! declare_loaders {
    (
        $(
            $creator:ident:  $name:ident < $impl_name:ident > for $model:ty => $key:ident ( $key_type:ty )
        ),+ $(,)?
    ) => {
        $(
            #[doc = concat!("Efficiently load [`", stringify!($model), "`]s in GraphQL queries/mutations")]
            pub(crate) type $name = DataLoader<$impl_name, NoCache>;

            #[doc = concat!("Create a new dataloader for [`", stringify!($model), "`]s")]
            pub(crate) fn $creator(db: &PgPool) -> $name {
                DataLoader::new($impl_name(db.clone()), tokio::task::spawn)
            }

            #[doc = concat!("The dataloader implementation for [`", stringify!($model), "`]s")]
            pub(crate) struct $impl_name(PgPool);

            #[async_trait]
            impl Loader<$key_type> for $impl_name {
                type Value = $model;
                type Error = Arc<database::Error>;

                async fn load(
                    &self,
                    keys: &[$key_type],
                ) -> Result<HashMap<$key_type, Self::Value>, Self::Error> {
                    Ok(<$model>::load(keys, &self.0)
                        .map_ok(|model| (model.$key.clone(), model))
                        .map_err(Arc::new)
                        .try_collect()
                        .await?)
                }
            }
        )+
    };
}

declare_loaders! {
    provider: ProviderLoader<ProviderLoaderImpl> for Provider => slug(String),
    user: UserLoader<UserLoaderImpl> for User => id(i32),
}
