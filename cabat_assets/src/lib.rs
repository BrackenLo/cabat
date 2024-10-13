//====================================================================

use asset_storage::{AssetLoadOptions, AssetManager};
use cabat_shipyard::{prelude::*, UniqueTools};
use downcast_rs::DowncastSync;

pub mod asset_loader;
pub mod asset_storage;
pub mod handle;

pub use anyhow::Result;
pub use asset_loader::RegisterAssetLoader;

//====================================================================

pub trait Asset: Send + Sync + DowncastSync {}

//====================================================================

pub struct AssetStoragePlugin;

impl Plugin for AssetStoragePlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .insert(AssetManager::new())
            .insert_default::<AssetLoadOptions>()
            .add_workload(Stages::Last, sys_update_storage);
    }
}

fn sys_update_storage(mut asset_storage: ResMut<AssetManager>) {
    asset_storage.update_handles();
}

//====================================================================
