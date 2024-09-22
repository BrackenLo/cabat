//====================================================================

use std::sync::Arc;

use shipyard::{AllStoragesView, Unique};
use shipyard_shared::{Size, WindowRaw, WindowResizeEvent, WindowSize};
use shipyard_tools::{EventHandler, ResMut, UniqueTools};

//====================================================================

#[derive(Unique)]
pub struct Window(Arc<winit::window::Window>);
impl Window {
    #[inline]
    pub fn inner(&self) -> &winit::window::Window {
        &self.0
    }

    #[inline]
    pub fn request_redraw(&self) {
        self.0.request_redraw();
    }

    // TODO - Window manipulation stuff here
}

//====================================================================

pub fn sys_add_window(window: Arc<winit::window::Window>, all_storages: AllStoragesView) {
    let size = Size::new(window.inner_size().width, window.inner_size().height);

    all_storages
        .insert(WindowSize::new(size))
        .insert(Window(window.clone()))
        .insert(WindowRaw::new(window.clone(), size));
}

pub fn sys_resize(
    new_size: Size<u32>,
    mut size: ResMut<WindowSize>,
    mut event_handler: ResMut<EventHandler>,
) {
    *size = WindowSize::new(new_size);

    event_handler.add_event(WindowResizeEvent::new(new_size));
}

//====================================================================
