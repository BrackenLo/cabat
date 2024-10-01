//====================================================================

use std::hash::{Hash, Hasher};

use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache};
use rustc_hash::FxHasher;
use shipyard::{Component, Unique};
use wgpu::util::DeviceExt;

use crate::{render_tools, Vertex};

use super::atlas::TextAtlas;

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct Text3dVertex {
    glyph_pos: [f32; 2],
    glyph_size: [f32; 2],
    uv_start: [f32; 2],
    uv_end: [f32; 2],
    color: u32,
}

impl Vertex for Text3dVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Float32x2,
            3 => Float32x2,
            4 => Uint32,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Text3dVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//====================================================================

#[derive(Unique)]
pub struct Text3dRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,

    pipeline: wgpu::RenderPipeline,
    buffer_bind_group_layout: wgpu::BindGroupLayout,
}

impl Text3dRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let atlas = TextAtlas::new(device);

        let buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text 3d Renderer Buffer Bind Group Layout"),
                entries: &[render_tools::bgl_uniform_entry(
                    0,
                    wgpu::ShaderStages::VERTEX,
                )],
            });

        let pipeline = render_tools::create_pipeline(
            device,
            config,
            "Text3dRenderer",
            &[
                camera_bind_group_layout,
                atlas.bind_group_layout(),
                &buffer_bind_group_layout,
            ],
            &[Text3dVertex::desc()],
            include_str!("../../shaders/text3d.wgsl"),
            render_tools::RenderPipelineDescriptor {
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                ..Default::default()
            }
            .with_depth_stencil(),
        );

        Self {
            font_system,
            swash_cache,
            atlas,
            pipeline,
            buffer_bind_group_layout,
        }
    }

    pub fn prep<'a, B>(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, buffers: B)
    where
        B: IntoIterator<Item = &'a mut Text3dBuffer>,
    {
        buffers.into_iter().for_each(|text3d_buffer| {
            text3d_buffer.text_buffer.layout_runs().for_each(|run| {
                let mut hasher = FxHasher::default();

                let cache_keys = run
                    .glyphs
                    .iter()
                    .filter_map(|glyph| {
                        let physical = glyph.physical((0., 0.), 1.);

                        self.atlas
                            .use_glyph(
                                device,
                                queue,
                                &mut self.font_system,
                                &mut self.swash_cache,
                                &physical.cache_key,
                            )
                            .ok()?;

                        physical.cache_key.hash(&mut hasher);
                        text3d_buffer.color.hash(&mut hasher);

                        Some((physical.cache_key, physical.x, physical.y))
                    })
                    .collect::<Vec<_>>();

                let vertex_hash = hasher.finish();

                if vertex_hash != text3d_buffer.vertex_hash {
                    println!("Updating text vertives");

                    let glyph_vertices = cache_keys
                        .into_iter()
                        .map(|(key, x, y)| {
                            let data = self.atlas.get_glyph_data(&key).unwrap();
                            Text3dVertex {
                                glyph_pos: [x as f32, y as f32],
                                glyph_size: data.size,
                                uv_start: data.uv_start,
                                uv_end: data.uv_end,
                                color: text3d_buffer.color.0,
                            }
                        })
                        .collect::<Vec<_>>();

                    render_tools::update_instance_buffer(
                        device,
                        queue,
                        "Text3d Vertex Buffer",
                        &mut text3d_buffer.vertex_buffer,
                        &mut text3d_buffer.vertex_count,
                        &glyph_vertices,
                    );

                    text3d_buffer.vertex_hash = vertex_hash;
                }
            });
        });
    }

    pub fn render<'a, B>(
        &self,
        pass: &mut wgpu::RenderPass,
        camera_bind_group: &wgpu::BindGroup,
        buffers: B,
    ) where
        B: IntoIterator<Item = &'a Text3dBuffer>,
    {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_bind_group(1, self.atlas.bind_group(), &[]);

        buffers.into_iter().for_each(|buffer| {
            pass.set_vertex_buffer(0, buffer.vertex_buffer.slice(..));
            pass.set_bind_group(2, &buffer.uniform_bind_group, &[]);
            pass.draw(0..4, 0..buffer.vertex_count);
        });
    }
}

//====================================================================

#[derive(Component)]
pub struct Text3dBuffer {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    vertex_hash: u64,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    pub text_buffer: Buffer,
    pub color: Color,
}

impl Text3dBuffer {
    pub fn new(device: &wgpu::Device, text3d_renderer: &mut Text3dRenderer) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text 3d Vertex Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let vertex_count = 0;

        let transform = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::ONE,
            glam::Quat::IDENTITY,
            glam::Vec3::new(0., 0., 20.),
        )
        .to_cols_array();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Text 3d Uniform Buffer"),
            contents: bytemuck::cast_slice(&[transform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text 3d Uniform Bind Group"),
            layout: &text3d_renderer.buffer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
            }],
        });

        let mut text_buffer = Buffer::new(
            &mut text3d_renderer.font_system,
            Metrics::relative(25., 1.2),
        );

        text_buffer.set_text(
            &mut text3d_renderer.font_system,
            "Hello World!",
            Attrs::new(),
            Shaping::Advanced,
        );

        let color = Color::rgb(255, 255, 255);

        Self {
            vertex_buffer,
            vertex_count,
            vertex_hash: 0,
            uniform_buffer,
            uniform_bind_group,

            text_buffer,
            color,
        }
    }
}

//====================================================================
