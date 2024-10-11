//====================================================================

use cabat_shipyard::{prelude::*, GetWorld, UniqueTools};
use downcast_rs::DowncastSync;

use crate::{asset_loader::AssetTypeLoader, asset_storage::AssetStorage};

pub mod asset_loader;
pub mod asset_storage;
pub mod handle;
pub mod loaders;

//====================================================================

pub trait Asset: Send + Sync + DowncastSync {}

//====================================================================

pub struct AssetStoragePlugin;

impl Plugin for AssetStoragePlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .insert_default::<AssetStorage>()
            .register_loader(loaders::TextLoader)
            .add_workload(Stages::Last, sys_update_storage);
    }
}

fn sys_update_storage(mut asset_storage: ResMut<AssetStorage>) {
    asset_storage.update_references();
}

//====================================================================

pub trait RegisterAssetLoader {
    fn register_loader(&self, loader: impl AssetTypeLoader) -> &Self;
}

impl<T: GetWorld> RegisterAssetLoader for T {
    fn register_loader(&self, loader: impl AssetTypeLoader) -> &Self {
        match self.get_world().get_unique::<&mut AssetStorage>() {
            Ok(mut storage) => storage.register_loader(loader),

            Err(shipyard::error::GetStorage::MissingStorage { .. }) => {
                let mut asset_storage = AssetStorage::new();
                asset_storage.register_loader(loader);
                self.get_world().add_unique(asset_storage);
            }

            Err(_) => unimplemented!(),
        };

        self
    }
}

//====================================================================
