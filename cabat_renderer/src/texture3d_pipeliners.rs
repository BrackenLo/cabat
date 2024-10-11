//====================================================================

use shipyard::Unique;

use crate::{
    render_tools,
    shared::{
        SharedPipelineResources, TextureRectVertex, TEXTURE_RECT_INDEX_COUNT, TEXTURE_RECT_INDICES,
        TEXTURE_RECT_VERTICES,
    },
    Vertex,
};

//====================================================================

// TODO - Create Texture3dPlugin - see Text3dPlugin
//      - Also see about renaming into Texture3dRenderer

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Texture3dInstanceRaw {
    pub size: [f32; 2],
    pub transform: [f32; 16],
    pub color: [f32; 4],
}

impl Vertex for Texture3dInstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            2 => Float32x2,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Texture3dInstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

impl Default for Texture3dInstanceRaw {
    fn default() -> Self {
        Self {
            size: [1.; 2],
            transform: glam::Mat4::IDENTITY.to_cols_array(),
            color: [1.; 4],
        }
    }
}

//====================================================================

pub struct Texture3dInstanceToRender<'a> {
    pub texture_bind_group: &'a wgpu::BindGroup,
    pub instance_buffer: &'a wgpu::Buffer,
    pub instance_count: u32,
}

//====================================================================

#[derive(Unique)]
pub struct Texture3dPipeline {
    pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl Texture3dPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shared: &SharedPipelineResources,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let pipeline = render_tools::create_pipeline(
            device,
            config,
            "Texture 3d Pipeline",
            &[camera_bind_group_layout, shared.texture_bind_group_layout()],
            &[TextureRectVertex::desc(), Texture3dInstanceRaw::desc()],
            include_str!("../shaders/texture3d.wgsl"),
            render_tools::RenderPipelineDescriptor::default()
                .with_depth_stencil()
                .with_backface_culling(),
        );

        let vertex_buffer =
            render_tools::vertex_buffer(device, "Texture 3d", &TEXTURE_RECT_VERTICES);
        let index_buffer = render_tools::index_buffer(device, "Texture 3d", &TEXTURE_RECT_INDICES);
        let index_count = TEXTURE_RECT_INDEX_COUNT;

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }

    pub fn render(
        &self,
        pass: &mut wgpu::RenderPass,
        camera_bind_group: &wgpu::BindGroup,
        instances: &[Texture3dInstanceToRender],
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);

        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        instances.into_iter().for_each(|instance| {
            pass.set_vertex_buffer(1, instance.instance_buffer.slice(..));
            pass.set_bind_group(1, instance.texture_bind_group, &[]);
            pass.draw_indexed(0..self.index_count, 0, 0..instance.instance_count);
        });
    }
}

//====================================================================
