//====================================================================

use std::hash::{Hash, Hasher};

use cosmic_text::{Attrs, Buffer, CacheKey, Color, FontSystem, Metrics, Shaping, SwashCache, Wrap};
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

struct LocalGlyphData {
    x: f32,
    y: f32,
    key: CacheKey,
    color: u32,
}

//--------------------------------------------------

// TODO - Try to move font system, swash cache and atlas out of renderer
//      - as can be used in other pipeline contexts
#[derive(Unique)]
pub struct Text3dRenderer {
    pipeline: wgpu::RenderPipeline,
    buffer_bind_group_layout: wgpu::BindGroupLayout,
}

impl Text3dRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        atlas: &TextAtlas,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
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
                fragment_targets: Some(&[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                })]),
                ..Default::default()
            }
            .with_depth_stencil(),
        );

        Self {
            pipeline,
            buffer_bind_group_layout,
        }
    }

    pub fn prep<'a, Buf>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        atlas: &mut TextAtlas,
        buffers: Buf,
    ) where
        Buf: IntoIterator<Item = &'a mut Text3dBuffer>,
    {
        buffers.into_iter().for_each(|text3d_buffer| {
            let mut rebuild_all_lines = false;
            // let mut rebuild_start_index = 0;

            let local_glyph_data = text3d_buffer
                .text_buffer
                .layout_runs()
                .enumerate()
                .flat_map(|(index, layout_run)| {
                    // Hasher for determining if a line has changed
                    let mut hasher = FxHasher::default();

                    let mut line_length = 0;

                    //--------------------------------------------------

                    // Iterate through each glyph in the line - prep and check
                    let local_glyph_data = layout_run
                        .glyphs
                        .iter()
                        .map(|glyph| {
                            let physical = glyph.physical((0., 0.), 1.);

                            // Try to prep glyph in atlas
                            if let Err(_) = atlas.use_glyph(
                                device,
                                queue,
                                font_system,
                                swash_cache,
                                &physical.cache_key,
                            ) {
                                todo!()
                                // panic!("TODO")
                                // return;
                            }

                            // Check if glyph has specific color to use
                            let color = match glyph.color_opt {
                                Some(color) => color,
                                None => text3d_buffer.color,
                            };

                            // Hash results to check changes
                            physical.cache_key.hash(&mut hasher);
                            color.hash(&mut hasher);

                            // Count number of glyphs in line
                            line_length += 1;

                            // Data for rebuilding later
                            LocalGlyphData {
                                x: physical.x as f32,
                                y: physical.y as f32 - layout_run.line_y,
                                key: physical.cache_key,
                                color: color.0,
                            }
                        })
                        .collect::<Vec<_>>();

                    //--------------------------------------------------

                    let line_hash = hasher.finish();

                    if text3d_buffer.lines.len() <= index {
                        text3d_buffer.lines.push(Text3dBufferLine::default());
                    }

                    let line_entry = &mut text3d_buffer.lines[index];

                    if line_hash != line_entry.hash {
                        println!("Line '{}' hash updated '{}'", index, line_hash);

                        line_entry.hash = line_hash;
                        line_entry.length = line_length;

                        rebuild_all_lines = true;
                    }

                    local_glyph_data

                    //--------------------------------------------------

                    // OPTIMIZE - The Real optimisations start here
                    // if rebuild_all_lines {
                    //     // Update and return
                    //     return;
                    // }

                    // let rebuild = match text3d_buffer.lines.get(index) {
                    //     Some(_) => todo!(),
                    //     None => true,
                    // };

                    // match (rebuild_all_lines, text3d_buffer.lines.get(index)) {
                    //     // Create entry and populate
                    //     (true, None) => todo!(),

                    //     // Update entry
                    //     (true, Some(_)) => todo!(),

                    //     // Create entry and populate and mark rebuild all lines with rebuild start index
                    //     (false, None) => todo!(),

                    //     // match entry with line hash. rebuild if required. if line length changed, mark rebuild all lines with rebuild start index
                    //     (false, Some(_)) => todo!(),
                    // };
                })
                .collect::<Vec<_>>();

            if rebuild_all_lines {
                let glyph_vertices = local_glyph_data
                    .into_iter()
                    .map(|local_data| {
                        let data = atlas.get_glyph_data(&local_data.key).unwrap();

                        let x = local_data.x + data.left + data.width / 2.;
                        let y = local_data.y + data.top; // TODO - Run Line

                        Text3dVertex {
                            glyph_pos: [x, y],
                            glyph_size: [data.width, data.height],
                            uv_start: data.uv_start,
                            uv_end: data.uv_end,
                            color: local_data.color,
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
            }
        });
    }

    pub fn render<'a, B>(
        &self,
        pass: &mut wgpu::RenderPass,
        atlas: &TextAtlas,
        camera_bind_group: &wgpu::BindGroup,
        buffers: B,
    ) where
        B: IntoIterator<Item = &'a Text3dBuffer>,
    {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_bind_group(1, atlas.bind_group(), &[]);

        buffers.into_iter().for_each(|buffer| {
            pass.set_vertex_buffer(0, buffer.vertex_buffer.slice(..));
            pass.set_bind_group(2, &buffer.uniform_bind_group, &[]);
            pass.draw(0..4, 0..buffer.vertex_count);
        });
    }
}

//====================================================================

pub struct Text3dBufferDescriptor<'a> {
    pub metrics: Metrics,
    pub word_wrap: Wrap,
    pub attributes: Attrs<'a>,
    pub text: &'a str,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub color: Color,

    pub pos: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl<'a> Default for Text3dBufferDescriptor<'a> {
    fn default() -> Self {
        Self {
            metrics: Metrics::relative(30., 1.2),
            word_wrap: Wrap::WordOrGlyph,
            attributes: Attrs::new(),
            text: "",
            width: Some(800.),
            height: None,
            color: Color::rgb(0, 0, 0),

            pos: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        }
    }
}

#[derive(Default)]
struct Text3dBufferLine {
    hash: u64,
    length: usize,
}

#[derive(Component)]
pub struct Text3dBuffer {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    lines: Vec<Text3dBufferLine>,

    // 3d Transform
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    pub text_buffer: Buffer,
    pub color: Color,
}

impl Text3dBuffer {
    pub fn new(
        device: &wgpu::Device,
        text3d_renderer: &mut Text3dRenderer,
        font_system: &mut FontSystem,
        desc: &Text3dBufferDescriptor,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text 3d Vertex Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let vertex_count = 0;
        let lines = Vec::new();

        let transform =
            glam::Mat4::from_scale_rotation_translation(desc.scale, desc.rotation, desc.pos)
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

        let mut text_buffer = Buffer::new(font_system, desc.metrics);

        text_buffer.set_size(font_system, desc.width, desc.height);
        text_buffer.set_wrap(font_system, desc.word_wrap);

        text_buffer.set_text(font_system, desc.text, desc.attributes, Shaping::Advanced);

        Self {
            vertex_buffer,
            vertex_count,
            lines,

            uniform_buffer,
            uniform_bind_group,

            text_buffer,
            color: desc.color,
        }
    }
}

//====================================================================
