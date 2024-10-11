//====================================================================

use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    asset_storage::{ReferenceCountSignal, Sender},
    Asset,
};

//====================================================================

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct HandleInner(u32);

impl HandleInner {
    #[inline]
    pub(crate) fn from_id(id: u32) -> Self {
        Self(id)
    }

    pub(crate) fn get_next(&mut self) -> Self {
        let id = *self;
        self.0 += 1;
        id
    }
}

//--------------------------------------------------

pub struct HandleId<A: Asset> {
    id: HandleInner,
    data: PhantomData<A>,
}

impl<A: Asset> HandleId<A> {
    pub(crate) fn new(id: HandleInner) -> Self {
        Self {
            id,
            data: PhantomData,
        }
    }
}

impl<A: Asset> Hash for HandleId<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<A: Asset> Clone for HandleId<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: Asset> Copy for HandleId<A> {}

impl<A: Asset> PartialEq for HandleId<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A: Asset> Eq for HandleId<A> {}

impl<A: Asset> Display for HandleId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", std::any::type_name::<A>(), self.id.0)
    }
}

impl<A: Asset> Debug for HandleId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HandleId(id: {:?}, data_type: {})",
            self.id,
            std::any::type_name::<A>()
        )
    }
}

//====================================================================

#[derive(Debug)]
pub struct Handle<A: Asset> {
    handle_id: HandleId<A>,
    sender: Sender,
    asset: Arc<RwLock<A>>,
}

impl<A: Asset> Handle<A> {
    pub(crate) fn new(handle_id: HandleId<A>, sender: Sender, asset: Arc<RwLock<A>>) -> Self {
        sender
            .send(ReferenceCountSignal::Increase(handle_id.id))
            .unwrap();

        Self {
            handle_id,
            sender,
            asset,
        }
    }

    #[inline]
    pub fn id(&self) -> HandleId<A> {
        self.handle_id
    }

    #[inline]
    pub fn inner(&self) -> RwLockReadGuard<A> {
        self.asset.read()
    }

    #[inline]
    pub fn inner_mut(&mut self) -> RwLockWriteGuard<A> {
        self.asset.write()
    }
}

impl<A: Asset> Clone for Handle<A> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.handle_id, self.sender.clone(), self.asset.clone())
    }
}

impl<A: Asset> PartialEq for Handle<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.handle_id.id == other.handle_id.id
    }
}

impl<A> Display for Handle<A>
where
    A: Asset + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle {}: {}", self.handle_id, self.asset.read())
    }
}

impl<A: Asset> Drop for Handle<A> {
    fn drop(&mut self) {
        // TODO - Better err handling
        if let Err(_) = self
            .sender
            .send(ReferenceCountSignal::Decrease(self.handle_id.id))
        {
            log::warn!(
                "Failed to send decrease signal on destruction of handle {:?}",
                self.handle_id
            );
        }
    }
}

//====================================================================
