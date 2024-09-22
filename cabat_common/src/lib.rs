//====================================================================

use std::{fmt::Display, sync::Arc};

use cabat_shipyard::Event;
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

#[derive(Unique)]
pub struct WindowSize(Size<u32>);

impl WindowSize {
    #[inline]
    pub fn new(size: Size<u32>) -> Self {
        Self(size)
    }

    #[inline]
    pub fn size(&self) -> Size<u32> {
        self.0
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.0.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.0.height
    }

    #[inline]
    pub fn width_f32(&self) -> f32 {
        self.0.width as f32
    }

    #[inline]
    pub fn height_f32(&self) -> f32 {
        self.0.height as f32
    }
}

#[derive(Event)]
pub struct WindowResizeEvent(Size<u32>);

impl WindowResizeEvent {
    #[inline]
    pub fn new(new_size: Size<u32>) -> Self {
        Self(new_size)
    }

    #[inline]
    pub fn size(&self) -> Size<u32> {
        self.0
    }
}

//====================================================================
