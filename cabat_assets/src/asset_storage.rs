//====================================================================

use std::{
    any::TypeId,
    collections::HashMap,
    fmt::{Debug, Display},
    hash::BuildHasherDefault,
    path::PathBuf,
    sync::Arc,
};

use cabat_shipyard::Res;
use crossbeam::channel::TryRecvError;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use rustc_hash::FxHasher;
use shipyard::{AllStoragesView, Unique};

use crate::{
    asset_loader::AssetLoader,
    handle::{Handle, HandleId},
    Asset,
};

//====================================================================

pub(crate) type Sender<A> = crossbeam::channel::Sender<ReferenceCountSignal<A>>;
pub(crate) type Receiver<A> = crossbeam::channel::Receiver<ReferenceCountSignal<A>>;
pub(crate) type Hasher = BuildHasherDefault<FxHasher>;

pub(crate) type Data<A> = Arc<A>;

//====================================================================

#[derive(thiserror::Error)]
pub enum AssetLoadError {
    FileDoesNotExist(PathBuf),
    IsNotFile(PathBuf),
    InvalidExtension,
    NoLoaderForExtension(String),    // Type Name, Ext
    InvalidCastType(String, String), // Type 1, Type 2

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Debug for AssetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetLoadError::FileDoesNotExist(path_buf) => {
                f.write_fmt(format_args!("File does not exist at path {:?}", path_buf))
            }

            AssetLoadError::IsNotFile(path_buf) => f.write_fmt(format_args!(
                "Provided path is not file at '{:?}'",
                path_buf
            )),

            AssetLoadError::InvalidExtension => f.write_str("Invalid extension provided"),

            AssetLoadError::NoLoaderForExtension(ext) => {
                f.write_fmt(format_args!("No loader for file extension '{}'", ext))
            }

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

//--------------------------------------------------

#[derive(Unique)]
pub struct AssetLoadOptions {
    load_path: PathBuf,
}

impl Default for AssetLoadOptions {
    fn default() -> Self {
        Self {
            load_path: match std::env::current_dir() {
                Ok(path) => path.join("res"),
                Err(_) => PathBuf::default(),
            },
        }
    }
}

//====================================================================

pub(crate) enum ReferenceCountSignal<A: Asset> {
    Increase(HandleId<A>),
    Decrease(HandleId<A>),
}

// pub enum AssetState<A> {
//     Loading,
//     Data(A),
// }

//====================================================================

#[derive(Unique)]
pub struct AssetManager {
    storages: HashMap<TypeId, Box<dyn AssetStorageAccess>, Hasher>,
}

impl AssetManager {
    pub(crate) fn new() -> Self {
        Self {
            storages: HashMap::default(),
        }
    }

    pub(crate) fn register_storage<A: Asset>(
        &mut self,
        storage: &AssetStorage<A>,
    ) -> Result<(), ()> {
        let id = TypeId::of::<A>();

        match self.storages.contains_key(&id) {
            true => Err(()),
            false => {
                self.storages.insert(id, Box::new(storage.clone_inner()));
                Ok(())
            }
        }
    }

    pub(crate) fn update_handles(&mut self) {
        self.storages.iter_mut().for_each(|(_key, value)| {
            value.update_handles();
        });
    }
}

//====================================================================

trait AssetStorageAccess: 'static + Send + Sync {
    // fn get_type_id(&self) -> TypeId;
    fn update_handles(&mut self);
}

impl<A: Asset> AssetStorageAccess for Arc<RwLock<AssetStorageInner<A>>> {
    // #[inline]
    // fn get_type_id(&self) -> TypeId {
    //     TypeId::of::<A>()
    // }

    #[inline]
    fn update_handles(&mut self) {
        self.write().update_handles();
    }
}

//====================================================================

// TODO - ID lookup with file path or label of some kind
//      - to skip loading / retreive already loaded assets
pub struct AssetStorageInner<A: Asset> {
    sender: Sender<A>,
    receiver: Receiver<A>,

    current_id: HandleId<A>,
    loaded_assets: HashMap<HandleId<A>, Data<A>, Hasher>,
    removed_assets: Vec<HandleId<A>>,
    handle_count: HashMap<HandleId<A>, u32, Hasher>,

    asset_loaders: Vec<Box<dyn AssetLoader<A>>>,
}

impl<A: Asset> AssetStorageInner<A> {
    fn update_handles(&mut self) {
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
                    match self.handle_count.get_mut(&handle_id) {
                        Some(count) => *count += 1,
                        None => unimplemented!(),
                    };
                }

                // Existing handle has been destroyed
                ReferenceCountSignal::Decrease(handle_id) => {
                    match self.handle_count.get_mut(&handle_id) {
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
            self.loaded_assets.remove(&handle_id);
            self.handle_count.remove(&handle_id);

            // TODO - Asset path removal
        });
    }
}

impl<A: Asset> AssetStorageInner<A> {
    fn insert_asset(&mut self, asset: A) -> Handle<A> {
        let asset = Arc::new(asset);
        let id = self.current_id.get_next();

        self.loaded_assets.insert(id, asset.clone());
        self.handle_count.insert(id, 0);

        log::trace!(
            "Creating new handle '{} - {}'",
            std::any::type_name::<A>(),
            id
        );

        Handle::new(id, self.sender.clone(), asset)
    }
}

//--------------------------------------------------

#[derive(Unique)]
pub struct AssetStorage<A: Asset> {
    inner: Arc<RwLock<AssetStorageInner<A>>>,
}

//--------------------------------------------------

impl<A: Asset> AssetStorage<A> {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();

        Self {
            inner: Arc::new(RwLock::new(AssetStorageInner {
                sender,
                receiver,

                current_id: HandleId::from_id(0),
                loaded_assets: HashMap::default(),
                removed_assets: Vec::new(),
                handle_count: HashMap::default(),

                asset_loaders: Vec::new(),
            })),
        }
    }

    #[inline]
    pub(crate) fn clone_inner(&self) -> Arc<RwLock<AssetStorageInner<A>>> {
        self.inner.clone()
    }

    #[inline]
    pub(crate) fn register_loader<L: AssetLoader<A>>(&mut self, loader: L) {
        self.inner.write().asset_loaders.push(Box::new(loader));
    }
}

//--------------------------------------------------

impl<A: Asset> AssetStorage<A> {
    pub fn get_asset(&self, id: impl Into<HandleId<A>>) -> Option<MappedRwLockReadGuard<A>> {
        RwLockReadGuard::try_map(self.inner.read(), |inner| {
            let asset = inner.loaded_assets.get(&id.into())?;
            Some(asset.as_ref())
        })
        .ok()
    }

    #[inline]
    pub fn get_storage(&self) -> MappedRwLockReadGuard<HashMap<HandleId<A>, Data<A>, Hasher>> {
        RwLockReadGuard::map(self.inner.read(), |inner| &inner.loaded_assets)
    }

    pub fn insert_asset(&self, asset: A) -> Handle<A> {
        log::trace!("Inserting asset of type '{}'", std::any::type_name::<A>());

        let mut inner = self.inner.write();
        inner.insert_asset(asset)
    }

    pub fn load_file(
        &self,
        all_storages: &AllStoragesView,
        path: impl Into<PathBuf>,
    ) -> crate::Result<Handle<A>> {
        let load_options = all_storages.borrow::<Res<AssetLoadOptions>>().unwrap();

        let path = load_options.load_path.join(path.into());

        log::trace!(
            "Loading file of type '{}' at path {:?}",
            std::any::type_name::<A>(),
            path
        );

        //--------------------------------------------------
        // Check file path

        let val = path
            .try_exists()
            .map_err(|_| AssetLoadError::FileDoesNotExist(path.clone()))?;

        anyhow::ensure!(val, AssetLoadError::FileDoesNotExist(path));
        anyhow::ensure!(path.is_file(), AssetLoadError::IsNotFile(path));

        let ext = path.extension().ok_or(AssetLoadError::InvalidExtension)?;
        let ext = ext.to_str().unwrap();

        //--------------------------------------------------

        std::mem::drop(load_options);

        //--------------------------------------------------
        // Load asset

        let asset = {
            let inner = self.inner.read();
            let loader = inner
                .asset_loaders
                .iter()
                .find(|loader| loader.extensions().contains(&ext))
                .ok_or(AssetLoadError::NoLoaderForExtension(ext.into()))?;

            loader.load_path(all_storages, path.as_path())?
        };

        let mut inner = self.inner.write();
        Ok(inner.insert_asset(asset))

        //--------------------------------------------------
    }

    // pub fn load_bytes(&self, all_storages: AllStoragesView, bytes: &[u8]) -> crate::Result<A> {
    //     todo!()
    // }
}

impl<A: Asset> Drop for AssetStorage<A> {
    fn drop(&mut self) {
        log::trace!(
            "Dropping asset storage '{}' and all handles.",
            std::any::type_name::<A>()
        );
        // TODO - Drop asset storage and all handles
    }
}

//====================================================================

//====================================================================
