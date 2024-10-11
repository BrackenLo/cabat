//====================================================================

use std::{
    any::TypeId,
    collections::HashMap,
    error::Error,
    fmt::{self, Debug, Display},
    hash::BuildHasherDefault,
    path::PathBuf,
    sync::Arc,
};

use crossbeam::channel::TryRecvError;
use parking_lot::RwLock;
use rustc_hash::FxHasher;
use shipyard::{AllStoragesView, Unique};

use crate::{
    asset_loader::{AssetLoaderOuter, AssetTypeLoader},
    handle::{Handle, HandleId, HandleInner},
    Asset,
};

//====================================================================

pub(crate) type Sender = crossbeam::channel::Sender<ReferenceCountSignal>;
pub(crate) type Receiver = crossbeam::channel::Receiver<ReferenceCountSignal>;
pub(crate) type Hasher = BuildHasherDefault<FxHasher>;

//====================================================================

pub(crate) enum ReferenceCountSignal {
    Increase(HandleInner),
    Decrease(HandleInner),
}

//--------------------------------------------------

#[derive(Debug)]
pub enum AssetStorageError {}

//--------------------------------------------------

#[derive(Clone)]
pub enum AssetLoadError {
    FileDoesNotExist(PathBuf),
    IsNotFile(PathBuf),
    InvalidExtension,
    NoLoaderForType(String, String), // Type Name, Ext
    InvalidCastType(String, String), // Type 1, Type 2
}

impl Error for AssetLoadError {}

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
        }
    }
}

impl Display for AssetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

//====================================================================

#[derive(Unique)]
pub struct AssetStorage {
    sender: Sender,
    receiver: Receiver,

    // Path to load assets from
    load_path: PathBuf,

    current_id: HandleInner,
    asset_loaders: HashMap<TypeId, Arc<dyn AssetLoaderOuter>, Hasher>,

    // TODO - use seperate hash maps for different assets
    loaded: HashMap<HandleInner, Arc<RwLock<dyn Asset>>, Hasher>,
    handle_count: HashMap<HandleInner, u32, Hasher>,
    removed_assets: Vec<HandleInner>,
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

            current_id: HandleInner::from_id(0),
            asset_loaders: HashMap::with_hasher(Hasher::default()),

            loaded: HashMap::with_hasher(Hasher::default()),
            handle_count: HashMap::with_hasher(Hasher::default()),
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
    // pub fn insert_data<T: Asset>(&mut self, data: T) -> Handle<T> {
    //     let id = self.current_id.get_next();

    //     let handle_inner = Arc::new(data);

    //     // Add reference to data to storage
    //     self.loaded.insert(id, handle_inner.clone());
    //     self.handle_count.insert(id, 0);

    //     // Construct Handle and return
    //     let handle_id = HandleId::new(id);
    //     let handle = Handle::new(handle_id, self.sender.clone(), handle_inner);
    //     handle
    // }

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

        let data = Arc::new(RwLock::new(*data));

        let id = self.current_id.get_next();

        self.loaded.insert(id, data.clone());
        self.handle_count.insert(id, 0);

        let handle_id = HandleId::new(id);
        let handle = Handle::new(handle_id, self.sender.clone(), data);

        Ok(handle)

        //--------------------------------------------------
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
                ReferenceCountSignal::Increase(handle_inner) => {
                    match self.handle_count.get_mut(&handle_inner) {
                        Some(count) => *count += 1,
                        None => unimplemented!(),
                    }
                }

                // Existing handle has been destroyed
                ReferenceCountSignal::Decrease(handle_inner) => {
                    match self.handle_count.get_mut(&handle_inner) {
                        Some(count) => {
                            *count -= 1;
                            if *count == 0 {
                                self.removed_assets.push(handle_inner);
                            }
                        }
                        None => unimplemented!(),
                    }
                }
            }
        }

        // Remove pending assets
        self.removed_assets.iter().for_each(|to_remove| {
            self.loaded.remove(&to_remove);
            self.handle_count.remove(&to_remove);

            // TODO - Asset path removal
        });
    }
}

//====================================================================
