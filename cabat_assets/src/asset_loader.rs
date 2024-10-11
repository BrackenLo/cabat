//====================================================================

use std::{any::Any, path::Path};

use shipyard::AllStoragesView;

use crate::{asset_storage::AssetLoadError, Asset};

//====================================================================

pub trait AssetTypeLoader: 'static + Send + Sync {
    type AssetType: Asset;

    fn load(&self, all_storages: AllStoragesView, path: &Path) -> crate::Result<Self::AssetType>;
    fn extensions(&self) -> &[&str];

    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self::AssetType>()
    }
}

//====================================================================

pub trait AssetLoaderOuter: 'static + Send + Sync {
    fn load(
        &self,
        all_storages: AllStoragesView,
        path: &Path,
    ) -> Result<LoadedAsset, AssetLoadError>;
    fn extensions(&self) -> &[&str];

    fn type_name(&self) -> &str;
}

impl<L> AssetLoaderOuter for L
where
    L: AssetTypeLoader + 'static + Send + Sync,
{
    #[inline]
    fn load(
        &self,
        all_storages: AllStoragesView,
        path: &Path,
    ) -> Result<LoadedAsset, AssetLoadError> {
        match L::load(&self, all_storages, path) {
            Ok(asset) => Ok(asset.into()),
            Err(_) => todo!(),
        }
    }

    #[inline]
    fn extensions(&self) -> &[&str] {
        L::extensions(&self)
    }

    #[inline]
    fn type_name(&self) -> &str {
        L::type_name(self)
    }
}

//====================================================================

pub struct LoadedAsset {
    // pub(crate) type_id: TypeId,
    pub(crate) type_name: String,
    pub(crate) data: Box<dyn Any + Send + Sync>,
}

impl<A: Asset> From<A> for LoadedAsset {
    fn from(value: A) -> Self {
        Self {
            // type_id: std::any::TypeId::of::<A>(),
            type_name: std::any::type_name::<A>().to_string(),
            data: Box::new(value),
        }
    }
}

//====================================================================
