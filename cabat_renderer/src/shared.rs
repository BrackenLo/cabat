//====================================================================

use shipyard::Unique;

use crate::{
    render_tools,
    texture::{RawTexture, Texture},
};

use super::Vertex;

//====================================================================

#[derive(Unique)]
pub struct SharedPipelineResources {
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl SharedPipelineResources {
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
