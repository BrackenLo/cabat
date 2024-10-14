//====================================================================

use std::path::Path;

use cabat_shipyard::{GetWorld, UniqueTools};
use shipyard::AllStoragesView;

use crate::{
    asset_storage::{AssetManager, AssetStorage},
    Asset,
};

//====================================================================

pub trait AssetLoader<A: Asset>: 'static + Send + Sync {
    fn load_path(&self, all_storages: &AllStoragesView, path: &Path) -> crate::Result<A>;
    fn extensions(&self) -> &[&str];

    // fn load_bytes(&self, all_storages: AllStoragesView, bytes: &[u8]) -> crate::Result<A>;
}

//====================================================================

pub trait RegisterAssetLoader {
    fn register_loader<A: Asset>(&self, loader: impl AssetLoader<A>) -> &Self;
}

impl<T: GetWorld> RegisterAssetLoader for T {
    fn register_loader<A: Asset>(&self, loader: impl AssetLoader<A>) -> &Self {
        let mut manager = self.get_world().get_or_insert(|| AssetManager::new());
        let mut storage = self.get_world().get_or_insert(|| AssetStorage::<A>::new());

        let result = manager.register_storage(&storage);
        result.unwrap();

        storage.register_loader(loader);

        self
    }
}

//====================================================================
