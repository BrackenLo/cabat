//====================================================================

use cabat_assets::{asset_storage::AssetStorage, handle::Handle};
use cabat_shipyard::Res;
use shipyard::{AllStoragesView, Unique};

use crate::{
    render_tools,
    renderers::model::{Mesh, ModelData, ModelVertex},
    texture::{RawTexture, Texture},
    Device, Queue,
};

use super::Vertex;

//====================================================================

#[derive(Unique)]
pub struct SharedRendererResources {
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl SharedRendererResources {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture 3d Bind Group Layout"),
                entries: &[
                    render_tools::bgl_texture_entry(0),
                    render_tools::bgl_sampler_entry(1),
                ],
            });

        Self {
            texture_bind_group_layout,
        }
    }

    #[inline]
    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        texture: &RawTexture,
        label: Option<&str>,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }

    pub fn load_texture(
        &self,
        device: &wgpu::Device,
        texture: RawTexture,
        label: Option<&str>,
    ) -> Texture {
        let bind_group = self.create_bind_group(device, &texture, label);
        Texture::new(texture, bind_group)
    }
}

//====================================================================

#[derive(Unique, Default)]
pub struct DefaultRendererAssets {
    texture: Option<Handle<Texture>>,
    cube: Option<Handle<ModelData>>,
}

impl DefaultRendererAssets {
    pub fn get_texture(&self) -> Option<Handle<Texture>> {
        match &self.texture {
            Some(texture) => Some(texture.clone()),
            None => None,
        }
    }

    pub fn get_cube(&self) -> Option<Handle<ModelData>> {
        match &self.cube {
            Some(cube) => Some(cube.clone()),
            None => None,
        }
    }
}

impl DefaultRendererAssets {
    pub fn load_texture(&mut self, all_storages: &AllStoragesView) {
        // Don't reload texture
        if let Some(_) = self.texture {
            return;
        }

        log::info!("Loading default Texture");

        let (device, queue, shared) = all_storages
            .borrow::<(Res<Device>, Res<Queue>, Res<SharedRendererResources>)>()
            .unwrap();

        let raw_texture = RawTexture::from_color(
            device.inner(),
            queue.inner(),
            [255, 255, 255],
            Some("Default Texture"),
            None,
        );

        let default_texture =
            shared.load_texture(device.inner(), raw_texture, Some("Default Texture"));

        let handle = {
            match all_storages.get_unique::<&AssetStorage<Texture>>() {
                Ok(storage) => storage.insert_asset(default_texture),

                Err(shipyard::error::GetStorage::MissingStorage { .. }) => {
                    let storage = AssetStorage::<Texture>::new();
                    let handle = storage.insert_asset(default_texture);
                    all_storages.add_unique(storage);
                    handle
                }

                Err(e) => panic!("{}", e),
            }
        };

        self.texture = Some(handle);
    }

    pub fn load_cube(&mut self, all_storages: &AllStoragesView) {
        self.load_texture(all_storages);

        // Don't reload cube
        if let Some(_) = self.cube {
            return;
        }

        log::info!("Loading default cube model");

        let device = all_storages.borrow::<Res<Device>>().unwrap();

        let mut vertices = DEFAULT_CUBE_VERTICES;
        render_tools::calculate_model_normals(&mut vertices, &DEFAULT_CUBE_INDICES);

        let vertex_buffer = render_tools::vertex_buffer(device.inner(), "Cube", &vertices);

        let index_buffer =
            render_tools::index_buffer(device.inner(), "Cube", &DEFAULT_CUBE_INDICES);

        let default_model = ModelData {
            meshes: vec![Mesh {
                vertex_buffer,
                index_buffer,
                index_count: DEFAULT_CUBE_INDICES.len() as u32,
                material: self.get_texture().unwrap(),
            }],
        };

        let handle = {
            match all_storages.get_unique::<&AssetStorage<ModelData>>() {
                Ok(storage) => storage.insert_asset(default_model),

                Err(shipyard::error::GetStorage::MissingStorage { .. }) => {
                    let storage = AssetStorage::<ModelData>::new();
                    let handle = storage.insert_asset(default_model);
                    all_storages.add_unique(storage);
                    handle
                }

                Err(e) => panic!("{}", e),
            }
        };

        self.cube = Some(handle);
    }
}

pub const DEFAULT_CUBE_VERTICES: [ModelVertex; 8] = [
    // Back 4 vertices
    // 0 - Top Right
    ModelVertex {
        position: [-0.5, 0.5, 0.5],
        uv: [0., 0.],
        normal: [0., 0., 0.],
    },
    // 1 - Bottom Right
    ModelVertex {
        position: [-0.5, -0.5, 0.5],
        uv: [0., 1.],
        normal: [0., 0., 0.],
    },
    // 2 - Bottom Left
    ModelVertex {
        position: [0.5, -0.5, 0.5],
        uv: [1., 1.],
        normal: [0., 0., 0.],
    },
    // 3 - Top Left
    ModelVertex {
        position: [0.5, 0.5, 0.5],
        uv: [1., 0.],
        normal: [0., 0., 0.],
    },
    //

    // Front 4 vertices
    // 4 - Top Left
    ModelVertex {
        position: [-0.5, 0.5, -0.5],
        uv: [1., 0.],
        normal: [0., 0., 0.],
    },
    // 5 - Bottom Left
    ModelVertex {
        position: [-0.5, -0.5, -0.5],
        uv: [1., 1.],
        normal: [0., 0., 0.],
    },
    // 6 - Bottom Right
    ModelVertex {
        position: [0.5, -0.5, -0.5],
        uv: [0., 1.],
        normal: [0., 0., 0.],
    },
    // 7 - Top Right
    ModelVertex {
        position: [0.5, 0.5, -0.5],
        uv: [0., 0.],
        normal: [0., 0., 0.],
    },
];

pub const DEFAULT_CUBE_INDICES: [u16; 36] = [
    4, 5, 6, 4, 6, 7, // Front Face
    3, 2, 1, 3, 1, 0, // Back Face
    0, 1, 5, 0, 5, 4, // Left Face
    7, 6, 2, 7, 2, 3, // Right Face
    0, 4, 7, 0, 7, 3, // Top Face
    5, 1, 2, 5, 2, 6, // Bottom Face
];

//====================================================================

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct TextureRectVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

impl Vertex for TextureRectVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
                0 => Float32x2, 1 => Float32x2
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextureRectVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

pub const TEXTURE_RECT_VERTICES: [TextureRectVertex; 4] = [
    TextureRectVertex {
        pos: [-0.5, 0.5],
        uv: [0., 0.],
    },
    TextureRectVertex {
        pos: [-0.5, -0.5],
        uv: [0., 1.],
    },
    TextureRectVertex {
        pos: [0.5, 0.5],
        uv: [1., 0.],
    },
    TextureRectVertex {
        pos: [0.5, -0.5],
        uv: [1., 1.],
    },
];

pub const TEXTURE_RECT_INDICES: [u16; 6] = [0, 1, 3, 0, 3, 2];
pub const TEXTURE_RECT_INDEX_COUNT: u32 = TEXTURE_RECT_INDICES.len() as u32;

//====================================================================
