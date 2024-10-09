//====================================================================

use shipyard::{AllStoragesView, Unique};

mod atlas;
mod text2d;
mod text3d;

pub use atlas::TextAtlas;
pub use text2d::{Text2dBuffer, Text2dBufferDescriptor, Text2dPlugin, TextPipeline};
pub use text3d::{Text3dBuffer, Text3dBufferDescriptor, Text3dRenderer};

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

fn sys_setup_text_components(all_storages: AllStoragesView) {
    all_storages.add_unique(TextFontSystem(cosmic_text::FontSystem::new()));
    all_storages.add_unique(TextSwashCache(cosmic_text::SwashCache::new()));
}

//====================================================================
