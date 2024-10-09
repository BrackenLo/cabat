//====================================================================

use cabat_common::{WindowResizeEvent, WindowSize};
use cabat_shipyard::prelude::*;
use glyphon::{
    Attrs, Buffer, Cache, Resolution, Shaping, TextArea, TextAtlas, TextBounds, TextRenderer,
    Viewport, Wrap,
};
use shipyard::{
    AllStoragesView, Component, IntoIter, IntoWorkload, SystemModificator, Unique, View,
    WorkloadModificator,
};

use crate::{Device, Queue, RenderEncoder, RenderPassDesc, SurfaceConfig};

//====================================================================

pub use glyphon::{Color, Metrics};

use super::{sys_setup_text_components, TextFontSystem, TextSwashCache};

//====================================================================

pub struct Text2dPlugin;

impl Plugin for Text2dPlugin {
    fn build(self, workload_builder: WorkloadBuilder) -> WorkloadBuilder {
        workload_builder
            .add_workload_first(
                Stages::Setup,
                (sys_setup_text_components, sys_setup_text_pipeline)
                    .into_workload()
                    .after_all("renderer_setup"),
            )
            .add_workload_last(Stages::Update, (sys_prep_text).into_workload())
            .add_workload_post(
                Stages::Render,
                sys_render
                    .skip_if_missing_unique::<RenderEncoder>()
                    .after_all(crate::sys_finish_main_render_pass),
            )
            .add_workload(Stages::Last, sys_trim_text_pipeline)
            .add_event::<WindowResizeEvent>((sys_resize_text_pipeline).into_workload())
    }
}

//====================================================================

#[derive(Unique)]
pub struct TextPipeline {
    renderer: TextRenderer,
    atlas: TextAtlas,
    viewport: Viewport,
}

impl TextPipeline {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, config.format);
        let viewport = Viewport::new(device, &cache);

        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            renderer,
            atlas,
            viewport,
        }
    }

    fn resize(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.viewport.update(queue, Resolution { width, height });
    }

    fn prep(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,

        data: Vec<TextArea>,
    ) -> Result<(), glyphon::PrepareError> {
        self.renderer.prepare(
            device,
            queue,
            font_system,
            &mut self.atlas,
            &self.viewport,
            data,
            swash_cache,
        )
    }

    pub fn render<'a: 'b, 'b>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .unwrap();
    }

    pub fn trim(&mut self) {
        self.atlas.trim();
    }
}

fn sys_setup_text_pipeline(
    all_storages: AllStoragesView,
    device: Res<Device>,
    queue: Res<Queue>,
    config: Res<SurfaceConfig>,
) {
    let pipeline = TextPipeline::new(device.inner(), queue.inner(), config.inner());
    all_storages.add_unique(pipeline);
}

fn sys_resize_text_pipeline(
    queue: Res<Queue>,
    size: Res<WindowSize>,

    mut text_pipeline: ResMut<TextPipeline>,
) {
    text_pipeline.resize(queue.inner(), size.width(), size.height());
}

fn sys_prep_text(
    device: Res<Device>,
    queue: Res<Queue>,

    mut text_pipeline: ResMut<TextPipeline>,
    mut font_system: ResMut<TextFontSystem>,
    mut swash_cache: ResMut<TextSwashCache>,
    v_buffers: View<Text2dBuffer>,
) {
    let data = v_buffers
        .iter()
        .map(|buffer| TextArea {
            buffer: &buffer.buffer,
            left: buffer.pos.0,
            top: buffer.pos.1,
            scale: 1.,
            bounds: buffer.bounds,
            default_color: buffer.color,
            custom_glyphs: &[],
        })
        .collect::<Vec<_>>();

    text_pipeline
        .prep(
            device.inner(),
            queue.inner(),
            font_system.inner_mut(),
            swash_cache.inner_mut(),
            data,
        )
        .unwrap();
}

fn sys_render(mut tools: ResMut<RenderEncoder>, pipeline: Res<TextPipeline>) {
    let mut pass = tools.begin_render_pass(RenderPassDesc::none());
    pipeline.render(&mut pass);
}

fn sys_trim_text_pipeline(mut text_pipeline: ResMut<TextPipeline>) {
    text_pipeline.trim();
}

//====================================================================

pub struct Text2dBufferDescriptor<'a> {
    pub metrics: Metrics,

    pub bounds_top: i32,
    pub bounds_bottom: i32,
    pub bounds_left: i32,
    pub bounds_right: i32,
    pub word_wrap: Wrap,

    pub text: &'a str,
    pub pos: (f32, f32),
    pub width: Option<f32>,
    pub height: Option<f32>,

    pub color: Color,
}

impl Default for Text2dBufferDescriptor<'_> {
    fn default() -> Self {
        Self {
            metrics: Metrics::relative(30., 1.2),

            bounds_left: 0,
            bounds_top: 0,
            bounds_right: 800,
            bounds_bottom: 300,

            word_wrap: Wrap::WordOrGlyph,

            text: "",
            pos: (0., 0.),
            width: Some(800.),
            height: None,

            color: glyphon::Color::rgb(0, 0, 0),
        }
    }
}

impl<'a> Text2dBufferDescriptor<'a> {
    pub fn new_text(text: &'a str) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }
}

#[derive(Component)]
pub struct Text2dBuffer {
    pub buffer: Buffer,
    pub bounds: TextBounds,
    pub pos: (f32, f32),
    pub color: glyphon::Color,
}

impl Text2dBuffer {
    pub fn new(font_system: &mut cosmic_text::FontSystem, desc: &Text2dBufferDescriptor) -> Self {
        let mut buffer = Buffer::new(font_system, desc.metrics);

        buffer.set_text(font_system, desc.text, Attrs::new(), Shaping::Advanced);

        buffer.set_wrap(font_system, desc.word_wrap);
        buffer.set_size(font_system, desc.width, desc.height);

        Self {
            buffer,
            bounds: TextBounds {
                left: desc.bounds_left,
                top: desc.bounds_top,
                right: desc.bounds_right,
                bottom: desc.bounds_bottom,
            },
            pos: desc.pos,
            color: desc.color,
        }
    }

    #[inline]
    pub fn set_text(&mut self, font_system: &mut cosmic_text::FontSystem, text: &str) {
        self.buffer
            .set_text(font_system, text, Attrs::new(), Shaping::Advanced);
    }

    #[inline]
    pub fn set_size(
        &mut self,
        font_system: &mut cosmic_text::FontSystem,
        width: Option<f32>,
        height: Option<f32>,
    ) {
        self.buffer.set_size(font_system, width, height);
    }

    #[inline]
    pub fn set_metrics_and_size(
        &mut self,
        font_system: &mut cosmic_text::FontSystem,
        metrics: Metrics,
        width: Option<f32>,
        height: Option<f32>,
    ) {
        self.buffer
            .set_metrics_and_size(font_system, metrics, width, height);
    }
}

//====================================================================
