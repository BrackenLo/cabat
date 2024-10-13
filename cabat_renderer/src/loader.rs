//====================================================================

use cabat_assets::asset_loader::AssetLoader;
use cabat_shipyard::Res;

use crate::{
    shared::SharedRendererResources,
    texture::{RawTexture, Texture},
    Device, Queue,
};

//====================================================================

pub struct TextureLoader;

impl AssetLoader<Texture> for TextureLoader {
    fn load_path(
        &self,
        all_storages: shipyard::AllStoragesView,
        path: &std::path::Path,
    ) -> cabat_assets::Result<Texture> {
        let name = match path.file_name() {
            Some(file_name) => file_name.to_str().unwrap(),
            None => "Loaded Texture",
        };

        let image_reader = image::ImageReader::open(&path)?;
        let image = image_reader.decode()?;

        let device = all_storages.borrow::<Res<Device>>()?;
        let queue = all_storages.borrow::<Res<Queue>>()?;

        let raw_texture =
            RawTexture::from_image(device.inner(), queue.inner(), &image, Some(&name), None);

        let shared = all_storages.borrow::<Res<SharedRendererResources>>()?;

        let texture = shared.load_texture(device.inner(), raw_texture, Some(&name));

        Ok(texture)
    }

    fn extensions(&self) -> &[&str] {
        &["png", "jpg"]
    }

    // fn load_bytes(
    //     &self,
    //     all_storages: shipyard::AllStoragesView,
    //     bytes: &[u8],
    // ) -> cabat_assets::Result<Texture> {
    //     todo!()
    // }
}

//====================================================================
