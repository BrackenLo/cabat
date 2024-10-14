//====================================================================

use std::path::Path;

use cabat_assets::asset_loader::AssetLoader;
use cabat_shipyard::Res;
use image::DynamicImage;

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
        all_storages: &shipyard::AllStoragesView,
        path: &std::path::Path,
    ) -> cabat_assets::Result<Texture> {
        all_storages.run_with_data(sys_load_texture_from_path, path)
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

pub fn sys_load_texture_from_path(
    path: &Path,
    device: Res<Device>,
    queue: Res<Queue>,
    shared: Res<SharedRendererResources>,
) -> cabat_assets::Result<Texture> {
    let name = match path.file_name() {
        Some(file_name) => file_name.to_str().unwrap(),
        None => "Loaded Texture",
    };

    let image_reader = image::ImageReader::open(&path)?;
    let image = image_reader.decode()?;

    let raw_texture =
        RawTexture::from_image(device.inner(), queue.inner(), &image, Some(&name), None);

    let texture = shared.load_texture(device.inner(), raw_texture, Some(&name));

    Ok(texture)
}

pub fn sys_load_texture_from_image(
    image: &DynamicImage,
    device: Res<Device>,
    queue: Res<Queue>,
    shared: Res<SharedRendererResources>,
) -> cabat_assets::Result<Texture> {
    let raw_texture = RawTexture::from_image(device.inner(), queue.inner(), &image, None, None);
    let texture = shared.load_texture(device.inner(), raw_texture, None);

    Ok(texture)
}

//====================================================================
