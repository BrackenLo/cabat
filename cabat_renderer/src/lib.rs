//====================================================================

use cabat_common::{Size, WindowRaw, WindowResizeEvent, WindowSize};
use cabat_shipyard::{prelude::*, UniqueTools};
use pollster::FutureExt;
use shipyard::{AllStoragesView, IntoWorkload, SystemModificator, Unique, WorkloadModificator};
use texture::DepthTexture;

pub mod camera;
pub mod render_tools;
pub mod shared;
pub mod text;
pub mod texture;
pub mod texture3d_pipeliners;

//====================================================================

pub mod plugins {
    pub use crate::{text::Text2dPlugin, text::Text3dPlugin, CoreRendererPlugin};
}

pub mod crates {
    pub use wgpu;
}

//--------------------------------------------------

pub struct FullRendererPlugin;

impl Plugin for FullRendererPlugin {
    fn build(self, builder: WorkloadBuilder) -> WorkloadBuilder {
        builder
            .add_plugin(CoreRendererPlugin)
            .add_plugin(plugins::Text2dPlugin)
            .add_plugin(plugins::Text3dPlugin)
    }
}

//====================================================================

pub struct CoreRendererPlugin;

impl Plugin for CoreRendererPlugin {
    fn build(self, builder: WorkloadBuilder) -> WorkloadBuilder {
        builder
            .add_workload_first(
                Stages::Setup,
                (
                    sys_setup_renderer_components,
                    sys_setup_misc,
                    texture::sys_setup_depth_texture,
                )
                    .into_sequential_workload()
                    .tag("renderer_setup"),
            )
            .add_workload_pre(
                Stages::Render,
                (sys_setup_encoder, sys_setup_render_pass).into_sequential_workload(),
            )
            .add_workload_post(Stages::Render, sys_finish_main_render_pass)
            .add_workload_last(
                Stages::Render,
                (sys_submit_encoder).into_workload().tag("submit_encoder"),
            )
            .add_event::<WindowResizeEvent>(
                (
                    sys_resize,
                    texture::sys_resize_depth_texture.skip_if_missing_unique::<DepthTexture>(),
                )
                    .into_workload(),
            )
    }
}

//====================================================================

pub trait Vertex: bytemuck::Pod {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

//====================================================================

#[derive(Unique)]
pub struct Device(wgpu::Device);
impl Device {
    #[inline]
    pub fn inner(&self) -> &wgpu::Device {
        &self.0
    }
}

#[derive(Unique)]
pub struct Queue(wgpu::Queue);
impl Queue {
    #[inline]
    pub fn inner(&self) -> &wgpu::Queue {
        &self.0
    }
}

#[derive(Unique)]
pub struct Surface(wgpu::Surface<'static>);
impl Surface {
    #[inline]
    pub fn inner(&self) -> &wgpu::Surface {
        &self.0
    }
}

#[derive(Unique)]
pub struct SurfaceConfig(wgpu::SurfaceConfiguration);
impl SurfaceConfig {
    #[inline]
    pub fn inner(&self) -> &wgpu::SurfaceConfiguration {
        &self.0
    }

    pub fn resize(&mut self, size: Size<u32>) {
        self.0.width = size.width;
        self.0.height = size.height;
    }
}

//====================================================================

#[derive(Unique)]
pub struct ClearColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for ClearColor {
    fn default() -> Self {
        Self {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.,
        }
    }
}

impl ClearColor {
    #[inline]
    fn to_array(&self) -> [f64; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

//====================================================================

fn sys_setup_renderer_components(all_storages: AllStoragesView, window: Res<WindowRaw>) {
    log::info!("Creating core wgpu renderer components.");

    let size = window.size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let surface = instance.create_surface(window.arc().clone()).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .block_on()
        .unwrap();

    log::debug!("Chosen device adapter: {:#?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .block_on()
        .unwrap();

    let surface_capabilities = surface.get_capabilities(&adapter);

    let surface_format = surface_capabilities
        .formats
        .iter()
        .find(|format| format.is_srgb())
        .copied()
        .unwrap_or(surface_capabilities.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoNoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: surface_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    all_storages
        .insert(Device(device))
        .insert(Queue(queue))
        .insert(Surface(surface))
        .insert(SurfaceConfig(config));
}

fn sys_setup_misc(all_storages: AllStoragesView, device: Res<Device>) {
    all_storages.add_unique(ClearColor::default());
    all_storages.add_unique(camera::MainCamera(camera::Camera::new(
        device.inner(),
        &camera::PerspectiveCamera::default(),
    )))
}

//====================================================================

fn sys_resize(
    device: Res<Device>,
    surface: Res<Surface>,
    mut config: ResMut<SurfaceConfig>,
    size: Res<WindowSize>,
) {
    config.resize(size.size());
    surface.inner().configure(device.inner(), config.inner());
}

//====================================================================

#[derive(Unique)]
pub struct RenderPass {
    pass: wgpu::RenderPass<'static>,
}

impl RenderPass {
    pub fn pass(&mut self) -> &mut wgpu::RenderPass<'static> {
        &mut self.pass
    }
}

pub struct RenderPassDesc<'a> {
    pub use_depth: Option<&'a wgpu::TextureView>,
    pub clear_color: Option<[f64; 4]>,
}

impl RenderPassDesc<'_> {
    pub fn none() -> Self {
        Self {
            use_depth: None,
            clear_color: None,
        }
    }
}

impl Default for RenderPassDesc<'_> {
    fn default() -> Self {
        Self {
            use_depth: None,
            clear_color: Some([0.2, 0.2, 0.2, 1.]),
        }
    }
}

#[derive(Unique)]
pub struct RenderEncoder {
    surface_texture: wgpu::SurfaceTexture,
    surface_view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

impl RenderEncoder {
    fn new(device: &wgpu::Device, surface: &wgpu::Surface) -> Result<Self, wgpu::SurfaceError> {
        let (surface_texture, surface_view) = match surface.get_current_texture() {
            Ok(texture) => {
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                (texture, view)
            }
            Err(e) => return Err(e),
        };

        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Command Encoder"),
        });

        Ok(RenderEncoder {
            surface_texture,
            surface_view,
            encoder,
        })
    }

    fn finish(self, queue: &wgpu::Queue) {
        queue.submit(Some(self.encoder.finish()));
        self.surface_texture.present();
    }

    pub fn begin_render_pass(&mut self, desc: RenderPassDesc) -> wgpu::RenderPass {
        // Clear the current depth buffer and use it.
        let depth_stencil_attachment = match desc.use_depth {
            Some(view) => Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            None => None,
        };

        let load = match desc.clear_color {
            Some(color) => wgpu::LoadOp::Clear(wgpu::Color {
                r: color[0],
                g: color[1],
                b: color[2],
                a: color[3],
            }),
            None => wgpu::LoadOp::Load,
        };

        let render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Tools Basic Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass
    }
}

fn sys_setup_encoder(all_storages: AllStoragesView, device: Res<Device>, surface: Res<Surface>) {
    let encoder = match RenderEncoder::new(device.inner(), surface.inner()) {
        Ok(encoder) => encoder,
        Err(_) => todo!(),
    };

    all_storages.add_unique(encoder);
}

fn sys_setup_render_pass(
    all_storages: AllStoragesView,
    mut tools: ResMut<RenderEncoder>,
    clear_color: Res<ClearColor>,
    depth: Res<DepthTexture>,
) {
    let pass = tools
        .begin_render_pass(RenderPassDesc {
            use_depth: Some(&depth.main_texture().view),
            clear_color: Some(clear_color.to_array()),
        })
        .forget_lifetime();

    all_storages.add_unique(RenderPass { pass });
}

fn sys_finish_main_render_pass(all_storages: AllStoragesView) {
    all_storages.remove_unique::<RenderPass>().ok();
}

fn sys_submit_encoder(all_storages: AllStoragesView, queue: Res<Queue>) {
    let encoder = all_storages.remove_unique::<RenderEncoder>().unwrap();
    encoder.finish(queue.inner());
}

//====================================================================
