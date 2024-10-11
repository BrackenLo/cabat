//====================================================================

use crate::{asset_loader::AssetTypeLoader, Asset};

//====================================================================

impl Asset for String {}

pub struct TextLoader;

impl AssetTypeLoader for TextLoader {
    type AssetType = String;

    fn load(
        &self,
        _all_storages: shipyard::AllStoragesView,
        path: &std::path::Path,
    ) -> anyhow::Result<Self::AssetType> {
        Ok(std::fs::read_to_string(path)?)
    }

    fn extensions(&self) -> &[&str] {
        &["txt"]
    }
}

//====================================================================
