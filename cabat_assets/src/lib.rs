//====================================================================

use asset_storage::AssetStorage;
use cabat_shipyard::{prelude::*, UniqueTools};
use downcast_rs::DowncastSync;

pub mod asset_loader;
pub mod asset_storage;
pub mod handle;
pub mod loaders;

//====================================================================

pub trait Asset: Send + Sync + DowncastSync {}

//====================================================================

pub struct AssetStoragePlugin;

impl Plugin for AssetStoragePlugin {
    fn build(self, builder: WorkloadBuilder) -> WorkloadBuilder {
        let mut storage = asset_storage::AssetStorage::new();
        storage.register_loader(loaders::TextLoader);
        builder.insert(storage);

        // builder.insert(asset_storage::AssetStorage::new());

        builder.add_workload(Stages::Last, sys_update_storage)
    }
}

fn sys_update_storage(mut asset_storage: ResMut<AssetStorage>) {
    asset_storage.update_references();
}

//====================================================================
