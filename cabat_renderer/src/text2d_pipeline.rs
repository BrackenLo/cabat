//====================================================================

use cabat_common::{WindowResizeEvent, WindowSize};
use cabat_shipyard::prelude::*;
use glyphon::{
    Attrs, Buffer, Cache, FontSystem, Resolution, Shaping, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport, Wrap,
};
use shipyard::{
    AllStoragesView, Component, IntoIter, IntoWorkload, SystemModificator, Unique, View,
    WorkloadModificator,
};

use crate::{RenderEncoder, RenderPassDesc};

use super::{Device, Queue, SurfaceConfig};

//====================================================================

pub use glyphon::{Color, Metrics};

//====================================================================

pub struct Text2dPlugin;

impl Plugin for Text2dPlugin {
    fn build(self, workload_builder: WorkloadBuilder) -> WorkloadBuilder {
        workload_builder
            .add_workload_first(
                Stages::Setup,
                (sys_setup_text_pipeline)
                    .into_workload()
                    .after_all("renderer_setup"),
            )
            .add_workload_last(Stages::Update, (sys_prep_text).into_workload())
            .add_workload_post(
                Stages::Render,
                (sys_render
                    .skip_if_missing_unique::<RenderEncoder>()
                    .after_all(super::sys_finish_main_render_pass))
                .into_workload(),
            )
            .add_workload(Stages::Last, (sys_trim_text_pipeline).into_workload())
            .add_event::<WindowResizeEvent>((sys_resize_text_pipeline).into_workload())
    }
}

//====================================================================

#[derive(Unique)]
pub struct TextPipeline {
    renderer: TextRenderer,
    font_system: FontSystem,
    swash_cache: SwashCache,
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
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let mut atlas = TextAtlas::new(device, queue, &cache, config.format);
        let viewport = Viewport::new(device, &cache);

        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            renderer,
            font_system,
            swash_cache,
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
        data: Vec<TextArea>,
    ) -> Result<(), glyphon::PrepareError> {
        self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            data,
            &mut self.swash_cache,
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
    v_buffers: View<TextBuffer>,
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
        .prep(device.inner(), queue.inner(), data)
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

pub struct TextBufferDescriptor<'a> {
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

impl Default for TextBufferDescriptor<'_> {
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

impl<'a> TextBufferDescriptor<'a> {
    pub fn new_text(text: &'a str) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }
}

#[derive(Component)]
pub struct TextBuffer {
    pub buffer: Buffer,
    pub bounds: TextBounds,
    pub pos: (f32, f32),
    pub color: glyphon::Color,
}

impl TextBuffer {
    pub fn new(text_pipeline: &mut TextPipeline, desc: &TextBufferDescriptor) -> Self {
        let mut buffer = Buffer::new(&mut text_pipeline.font_system, desc.metrics);

        buffer.set_text(
            &mut text_pipeline.font_system,
            desc.text,
            Attrs::new(),
            Shaping::Advanced,
        );

        buffer.set_wrap(&mut text_pipeline.font_system, desc.word_wrap);
        buffer.set_size(&mut text_pipeline.font_system, desc.width, desc.height);

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
    pub fn set_text(&mut self, text_pipeline: &mut TextPipeline, text: &str) {
        self.buffer.set_text(
            &mut text_pipeline.font_system,
            text,
            Attrs::new(),
            Shaping::Advanced,
        );
    }

    #[inline]
    pub fn set_size(
        &mut self,
        text_pipeline: &mut TextPipeline,
        width: Option<f32>,
        height: Option<f32>,
    ) {
        self.buffer
            .set_size(&mut text_pipeline.font_system, width, height);
    }

    #[inline]
    pub fn set_metrics_and_size(
        &mut self,
        text_pipeline: &mut TextPipeline,
        metrics: Metrics,
        width: Option<f32>,
        height: Option<f32>,
    ) {
        self.buffer
            .set_metrics_and_size(&mut text_pipeline.font_system, metrics, width, height);
    }
}

//====================================================================
