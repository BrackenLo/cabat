//====================================================================

use std::{
    any::TypeId,
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
};

use crate::{
    asset_storage::{ReferenceCountSignal, Sender},
    Asset,
};

//====================================================================

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct HandleId {
    id: u32,
    type_id: TypeId,
}

impl HandleId {
    #[inline]
    pub(crate) fn from_id<A: Asset>(id: u32) -> Self {
        Self {
            id,
            type_id: TypeId::of::<A>(),
        }
    }

    pub(crate) fn get_next(&mut self) -> Self {
        let id = *self;
        self.id += 1;
        id
    }

    #[inline]
    pub(crate) fn get_type_id(&self) -> TypeId {
        self.type_id
    }
}

// TODO
impl Display for HandleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}", self.id)
    }
}

impl<A: Asset> From<Handle<A>> for HandleId {
    #[inline]
    fn from(value: Handle<A>) -> Self {
        value.handle_id
    }
}

//====================================================================

#[derive(Debug)]
pub struct Handle<A: Asset> {
    handle_id: HandleId,
    sender: Sender,
    asset: Arc<A>,
}

impl<A: Asset> Handle<A> {
    pub(crate) fn new(handle_id: HandleId, sender: Sender, asset: Arc<A>) -> Self {
        log::trace!(
            "Creating new handle '{} - {}'",
            std::any::type_name::<A>(),
            handle_id
        );

        sender
            .send(ReferenceCountSignal::Increase(handle_id))
            .unwrap();

        Self {
            handle_id,
            sender,
            asset,
        }
    }

    #[inline]
    pub fn id(&self) -> HandleId {
        self.handle_id
    }

    #[inline]
    pub fn inner(&self) -> &A {
        self.asset.as_ref()
    }

    // #[inline]
    // pub fn inner(&self) -> RwLockReadGuard<A> {
    //     self.asset.read()
    // }

    // #[inline]
    // pub fn inner_mut(&mut self) -> RwLockWriteGuard<A> {
    //     self.asset.write()
    // }
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
        write!(
            f,
            "Handle '{} - {}': {}",
            std::any::type_name::<A>(),
            self.handle_id,
            self.asset
        )
    }
}

impl<A: Asset> Drop for Handle<A> {
    fn drop(&mut self) {
        log::trace!(
            "Dropping handle '{} - {}'",
            std::any::type_name::<A>(),
            self.handle_id
        );

        // TODO - Better err handling
        if let Err(_) = self
            .sender
            .send(ReferenceCountSignal::Decrease(self.handle_id))
        {
            log::warn!(
                "Failed to send decrease signal on destruction of handle {:?}",
                self.handle_id
            );
        }
    }
}

//====================================================================
