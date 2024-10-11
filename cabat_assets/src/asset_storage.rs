//====================================================================

use std::{
    any::TypeId,
    collections::HashMap,
    fmt::{self, Debug, Display},
    hash::BuildHasherDefault,
    path::PathBuf,
    sync::Arc,
};

use crossbeam::channel::TryRecvError;
use rustc_hash::FxHasher;
use shipyard::{AllStoragesView, Unique};

use crate::{
    asset_loader::{AssetLoaderOuter, AssetTypeLoader},
    handle::{Handle, HandleId},
    Asset,
};

//====================================================================

pub(crate) type Sender = crossbeam::channel::Sender<ReferenceCountSignal>;
pub(crate) type Receiver = crossbeam::channel::Receiver<ReferenceCountSignal>;
pub(crate) type Hasher = BuildHasherDefault<FxHasher>;

//====================================================================

pub(crate) enum ReferenceCountSignal {
    Increase(HandleId),
    Decrease(HandleId),
}

//--------------------------------------------------

#[derive(Debug)]
pub enum AssetStorageError {}

//--------------------------------------------------

#[derive(thiserror::Error)]
pub enum AssetLoadError {
    FileDoesNotExist(PathBuf),
    IsNotFile(PathBuf),
    InvalidExtension,
    NoLoaderForType(String, String), // Type Name, Ext
    InvalidCastType(String, String), // Type 1, Type 2

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Debug for AssetLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetLoadError::FileDoesNotExist(path_buf) => {
                f.write_fmt(format_args!("File does not exist at path '{:?}'", path_buf))
            }

            AssetLoadError::IsNotFile(path_buf) => f.write_fmt(format_args!(
                "Provided path is not file at '{:?}'",
                path_buf
            )),

            AssetLoadError::InvalidExtension => f.write_str("Invalid extension provided"),

            AssetLoadError::NoLoaderForType(type_name, ext) => f.write_fmt(format_args!(
                "No loader for provided type '{}' and file extension '{}'",
                type_name, ext
            )),

            AssetLoadError::InvalidCastType(type_id, type_id1) => f.write_fmt(format_args!(
                "Cannot create asset of type '{:?}' from loaded type '{:?}'",
                type_id, type_id1
            )),

            AssetLoadError::Other(e) => f.write_fmt(format_args!("{}", e)),
        }
    }
}

impl Display for AssetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

//====================================================================

struct InnerStorage {
    current_id: HandleId,
    // asset_type: TypeId,
    loaded_assets: HashMap<HandleId, Arc<dyn Asset>, Hasher>,
    handle_count: HashMap<HandleId, u32, Hasher>,
}
impl InnerStorage {
    fn new<A: Asset>() -> Self {
        Self {
            current_id: HandleId::from_id::<A>(0),
            // asset_type: std::any::TypeId::of::<A>(),
            loaded_assets: HashMap::default(),
            handle_count: HashMap::default(),
        }
    }

    fn insert_data(&mut self, data: Arc<dyn Asset>) -> HandleId {
        let id = self.current_id.get_next();

        self.loaded_assets.insert(id, data);
        self.handle_count.insert(id, 0);

        id
    }
}

//--------------------------------------------------

// TODO - Track asset paths
#[derive(Unique)]
pub struct AssetStorage {
    sender: Sender,
    receiver: Receiver,

    // Path to load assets from
    load_path: PathBuf,

    asset_loaders: HashMap<TypeId, Arc<dyn AssetLoaderOuter>, Hasher>,
    storages: HashMap<TypeId, InnerStorage, Hasher>,

    removed_assets: Vec<HandleId>,
}

impl Default for AssetStorage {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();

        let load_path = match std::env::current_dir() {
            Ok(path) => path.join("res"),
            Err(_) => PathBuf::default(),
        };

        Self {
            sender,
            receiver,

            load_path,

            asset_loaders: HashMap::default(),
            storages: HashMap::default(),

            removed_assets: Vec::new(),
        }
    }
}

impl AssetStorage {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register_loader<L: AssetTypeLoader>(&mut self, loader: L) {
        let type_id = std::any::TypeId::of::<L::AssetType>();
        self.asset_loaders.insert(type_id, Arc::new(loader));
    }
}

//====================================================================

impl AssetStorage {
    pub fn load_file<'a, A>(
        &mut self,
        all_storages: AllStoragesView,
        path: impl Into<PathBuf>,
    ) -> Result<Handle<A>, AssetLoadError>
    where
        A: Asset,
    {
        let path = self.load_path.join(path.into());
        let type_id = std::any::TypeId::of::<A>();
        let type_name = std::any::type_name::<A>();

        //--------------------------------------------------
        // Check file path

        let val = path
            .try_exists()
            .map_err(|_| AssetLoadError::FileDoesNotExist(path.clone()))?;

        if !val {
            return Err(AssetLoadError::FileDoesNotExist(path));
        }

        // Ensure is file
        if !path.is_file() {
            return Err(AssetLoadError::IsNotFile(path));
        }

        let ext = path.extension().ok_or(AssetLoadError::InvalidExtension)?;

        //--------------------------------------------------
        // Load asset

        let loader = self
            .asset_loaders
            .iter()
            .find(|(id, val)| **id == type_id && val.extensions().contains(&ext.to_str().unwrap()));

        let (_, loader) = match loader {
            Some(loader) => loader,
            None => {
                return Err(AssetLoadError::NoLoaderForType(
                    type_name.to_string(),
                    format!("{:?}", ext),
                ))
            }
        };

        let loaded_asset = loader.load(all_storages, path.as_path())?;

        //--------------------------------------------------
        // Convert data and create handle

        let data: Box<A> = loaded_asset.data.downcast().map_err(|_| {
            AssetLoadError::InvalidCastType(loaded_asset.type_name, type_name.to_string())
        })?;

        let data = Arc::new(*data);

        let storage = self
            .storages
            .entry(type_id)
            .or_insert(InnerStorage::new::<A>());

        let handle_id = storage.insert_data(data.clone());
        let handle = Handle::new(handle_id, self.sender.clone(), data);

        Ok(handle)

        //--------------------------------------------------
    }

    pub fn get_storage<A: Asset>(&self) -> Option<&HashMap<HandleId, Arc<dyn Asset>, Hasher>> {
        let id = TypeId::of::<A>();
        let storage = self.storages.get(&id)?;

        Some(&storage.loaded_assets)
    }

    pub fn get_asset<A: Asset>(&self, id: impl Into<HandleId>) -> Option<&A> {
        let id: HandleId = id.into();

        assert!(id.get_type_id() == std::any::TypeId::of::<A>());

        let storage = self.storages.get(&id.get_type_id())?;

        let asset_any = storage.loaded_assets.get(&id)?;
        let value = asset_any.as_ref().as_any().downcast_ref::<A>().unwrap();

        Some(value)
    }
}

//====================================================================

impl AssetStorage {
    pub(crate) fn update_references(&mut self) {
        self.removed_assets.clear();

        // Loop through each received signal
        loop {
            let data = match self.receiver.try_recv() {
                Ok(data) => data,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    panic!("Error: Handle has been disconnected from Asset Storage")
                }
            };

            match data {
                // New handle has been created
                ReferenceCountSignal::Increase(handle_id) => {
                    let storage = match self.storages.get_mut(&handle_id.get_type_id()) {
                        Some(storage) => storage,
                        None => unimplemented!(),
                    };

                    match storage.handle_count.get_mut(&handle_id) {
                        Some(count) => *count += 1,
                        None => unimplemented!(),
                    };
                }

                // Existing handle has been destroyed
                ReferenceCountSignal::Decrease(handle_id) => {
                    let storage = match self.storages.get_mut(&handle_id.get_type_id()) {
                        Some(storage) => storage,
                        None => unimplemented!(),
                    };

                    match storage.handle_count.get_mut(&handle_id) {
                        Some(count) => {
                            *count -= 1;
                            if *count == 0 {
                                self.removed_assets.push(handle_id);
                            }
                        }
                        None => unimplemented!(),
                    }
                }
            }
        }

        // Remove pending assets
        self.removed_assets.iter().for_each(|handle_id| {
            let storage = match self.storages.get_mut(&handle_id.get_type_id()) {
                Some(storage) => storage,
                None => unimplemented!(),
            };

            storage.loaded_assets.remove(&handle_id);
            storage.handle_count.remove(&handle_id);

            // TODO - Asset path removal
        });
    }
}

impl Drop for AssetStorage {
    fn drop(&mut self) {
        log::trace!("Dropping asset storage and all handles");
        // TODO - Drop all handles here else lots of warnings
    }
}

//====================================================================
