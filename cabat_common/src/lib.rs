//====================================================================

use std::{fmt::Display, sync::Arc};

use shipyard::Unique;
use window_handles::WindowHandle;

mod window_handles;

//====================================================================

#[derive(Clone, Copy, Debug)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    #[inline]
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

impl<T> From<(T, T)> for Size<T> {
    #[inline]
    fn from(value: (T, T)) -> Self {
        Self {
            width: value.0,
            height: value.1,
        }
    }
}

impl<T: Display> Display for Size<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.width, self.height)
    }
}

//====================================================================

#[derive(Unique)]
pub struct WindowRaw {
    window: Arc<dyn WindowHandle>,
    size: Size<u32>,
}

impl WindowRaw {
    pub fn new(window: Arc<dyn WindowHandle>, size: Size<u32>) -> Self {
        Self { window, size }
    }

    pub fn arc(&self) -> &Arc<dyn WindowHandle> {
        &self.window
    }

    pub fn size(&self) -> Size<u32> {
        self.size
    }
}

//====================================================================
