//====================================================================

use cabat_shipyard::Res;
use shipyard::{AllStoragesView, Unique};

use crate::Device;

mod atlas;
mod text2d;
mod text3d;

pub use atlas::TextAtlas;
pub use cosmic_text::{Attrs, Color, Metrics};
pub use text2d::{Text2dBuffer, Text2dBufferDescriptor, Text2dPlugin, Text2dRenderer};
pub use text3d::{Text3dBuffer, Text3dBufferDescriptor, Text3dPlugin, Text3dRenderer};

//====================================================================

#[derive(Unique)]
pub struct TextFontSystem(cosmic_text::FontSystem);
impl TextFontSystem {
    pub fn inner(&self) -> &cosmic_text::FontSystem {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.0
    }
}

#[derive(Unique)]
pub struct TextSwashCache(cosmic_text::SwashCache);
impl TextSwashCache {
    pub fn inner(&self) -> &cosmic_text::SwashCache {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut cosmic_text::SwashCache {
        &mut self.0
    }
}

//====================================================================

fn sys_setup_text_components(all_storages: AllStoragesView, device: Res<Device>) {
    all_storages.add_unique(TextFontSystem(cosmic_text::FontSystem::new()));
    all_storages.add_unique(TextSwashCache(cosmic_text::SwashCache::new()));
    all_storages.add_unique(TextAtlas::new(device.inner()));
}

//====================================================================
