//====================================================================

use std::{hash::Hash, marker::PhantomData};

use crate::{
    asset_storage::{Data, ReferenceCountSignal, Sender},
    Asset,
};

//====================================================================

pub struct HandleId<A: Asset> {
    id: u32,
    phantom: PhantomData<A>,
}

impl<A: Asset> HandleId<A> {
    #[inline]
    pub(crate) fn from_id(id: u32) -> Self {
        Self {
            id,
            phantom: PhantomData,
        }
    }

    pub(crate) fn get_next(&mut self) -> Self {
        let id = *self;
        self.id += 1;
        id
    }
}

impl<A: Asset> Clone for HandleId<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            phantom: PhantomData,
        }
    }
}

impl<A: Asset> Copy for HandleId<A> {}

impl<A: Asset> Hash for HandleId<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<A: Asset> PartialEq for HandleId<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A: Asset> Eq for HandleId<A> {}

impl<A: Asset> std::fmt::Display for HandleId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}", self.id)
    }
}

impl<A: Asset> std::fmt::Debug for HandleId<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl<A: Asset> From<Handle<A>> for HandleId<A> {
    #[inline]
    fn from(value: Handle<A>) -> Self {
        value.id
    }
}

//====================================================================

#[derive(Debug)]
pub struct Handle<A: Asset> {
    id: HandleId<A>,
    sender: Sender<A>,
    asset: Data<A>,
}

impl<A: Asset> Handle<A> {
    pub(crate) fn new(id: HandleId<A>, sender: Sender<A>, asset: Data<A>) -> Self {
        log::trace!(
            "Creating new handle '{} - {}'",
            std::any::type_name::<A>(),
            id
        );

        sender.send(ReferenceCountSignal::Increase(id)).unwrap();

        Self { id, sender, asset }
    }

    #[inline]
    pub fn id(&self) -> HandleId<A> {
        self.id
    }

    #[inline]
    pub fn inner(&self) -> &A {
        self.asset.as_ref()
    }
}

impl<A: Asset> Clone for Handle<A> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.id, self.sender.clone(), self.asset.clone())
    }
}

impl<A: Asset> PartialEq for Handle<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id.id == other.id.id
    }
}

impl<A> std::fmt::Display for Handle<A>
where
    A: Asset + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Handle '{} - {}': {}",
            std::any::type_name::<A>(),
            self.id,
            self.asset
        )
    }
}

impl<A: Asset> Drop for Handle<A> {
    fn drop(&mut self) {
        log::trace!(
            "Dropping handle '{} - {}'",
            std::any::type_name::<A>(),
            self.id
        );

        // TODO - Better err handling
        if let Err(_) = self.sender.send(ReferenceCountSignal::Decrease(self.id)) {
            log::warn!(
                "Failed to send decrease signal on destruction of handle {:?}",
                self.id
            );
        }
    }
}

//====================================================================
