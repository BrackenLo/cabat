//====================================================================

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};

use cabat_assets::{
    asset_storage::AssetStorage,
    handle::{Handle, HandleId},
};
use cabat_shipyard::prelude::*;
use cabat_spatial::Transform;
use rustc_hash::FxHasher;
use shipyard::{AllStoragesView, Component, IntoIter, Unique, View};

use crate::{
    camera::MainCamera,
    render_tools,
    shared::{
        SharedRendererResources, TextureRectVertex, TEXTURE_RECT_INDEX_COUNT, TEXTURE_RECT_INDICES,
        TEXTURE_RECT_VERTICES,
    },
    texture::{RawTexture, Texture},
    Device, Queue, RenderPass, SurfaceConfig, Vertex,
};

//====================================================================

pub struct Texture3dPlugin;

impl Plugin for Texture3dPlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .add_workload_pre(Stages::Setup, sys_setup_texture_renderer)
            .add_workload_last(Stages::Update, sys_prep_texture3d)
            .add_workload(Stages::Render, sys_render_texture3d);
    }
}

fn sys_setup_texture_renderer(
    all_storages: AllStoragesView,
    device: Res<Device>,
    queue: Res<Queue>,
    config: Res<SurfaceConfig>,
    shared: Res<SharedRendererResources>,
    camera: Res<MainCamera>,
) {
    let renderer = Texture3dRenderer::new(
        device.inner(),
        queue.inner(),
        config.inner(),
        &shared,
        camera.bind_group_layout(),
    );

    all_storages.add_unique(renderer);
}

fn sys_prep_texture3d(
    device: Res<Device>,
    queue: Res<Queue>,
    mut renderer: ResMut<Texture3dRenderer>,
    v_sprite: View<Sprite>,
    v_transform: View<Transform>,
) {
    #[derive(PartialEq, Eq, Hash)]
    enum InstanceType {
        Texture(HandleId<Texture>),
        Default,
    }

    let instances =
        (&v_transform, &v_sprite)
            .iter()
            .fold(HashMap::new(), |mut acc, (transform, sprite)| {
                let instance = Texture3dInstanceRaw {
                    size: [sprite.width, sprite.height],
                    transform: transform.to_array(),
                    color: sprite.color,
                };

                let instance_type = match &sprite.texture {
                    Some(texture) => InstanceType::Texture(texture.id()),
                    None => InstanceType::Default,
                };

                acc.entry(instance_type)
                    .or_insert(Vec::new())
                    .push(instance);

                acc
            });

    let mut previous = renderer
        .instances
        .keys()
        .map(|id| *id)
        .collect::<HashSet<_>>();

    let mut default_used = false;

    instances.into_iter().for_each(|(id, raw)| {
        match id {
            InstanceType::Texture(handle_id) => {
                previous.remove(&handle_id);

                renderer
                    .instances
                    .entry(handle_id)
                    .and_modify(|instance| {
                        instance.update(device.inner(), queue.inner(), raw.as_slice());
                    })
                    .or_insert(Texture3dInstance {
                        instance_buffer: render_tools::create_instance_buffer(
                            device.inner(),
                            "Texture 3d",
                            raw.as_slice(),
                        ),
                        instance_count: raw.len() as u32,
                    });
            }

            InstanceType::Default => {
                default_used = true;

                renderer
                    .default_instances
                    .update(device.inner(), queue.inner(), raw.as_slice());
            }
        };
    });

    previous.into_iter().for_each(|to_remove| {
        renderer.instances.remove(&to_remove);
    });

    // Reset default instances if not in use
    if !default_used && renderer.default_instances.instance_count != 0 {
        renderer.default_instances.instance_buffer =
            device.inner().create_buffer(&wgpu::BufferDescriptor {
                label: Some("Default Texture 3d Instance Buffer"),
                size: 0,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            });

        renderer.default_instances.instance_count = 0;
    }
}

fn sys_render_texture3d(
    mut pass: ResMut<RenderPass>,
    renderer: Res<Texture3dRenderer>,
    camera: Res<MainCamera>,

    storage: Res<AssetStorage<Texture>>,
) {
    let use_default = match renderer.default_instances.instance_count != 0 {
        true => Some((
            None,
            &renderer.default_instances.instance_buffer,
            renderer.default_instances.instance_count,
        )),
        false => None,
    };

    let instances = renderer
        .instances
        .iter()
        .map(|(id, instance)| {
            (
                Some(*id),
                &instance.instance_buffer,
                instance.instance_count,
            )
        })
        .chain(use_default)
        .collect::<Vec<_>>();

    renderer.render_storage(
        pass.pass(),
        camera.bind_group(),
        instances.as_slice(),
        &storage,
    );
}

//====================================================================

#[derive(Component)]
pub struct Sprite {
    pub texture: Option<Handle<Texture>>,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
}

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
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
pub struct Texture3dRenderer {
    pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    instances: HashMap<HandleId<Texture>, Texture3dInstance, BuildHasherDefault<FxHasher>>,
    default_texture_bind_group: wgpu::BindGroup,
    default_instances: Texture3dInstance,
}

impl Texture3dRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        shared: &SharedRendererResources,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let pipeline = render_tools::create_pipeline(
            device,
            config,
            "Texture 3d Pipeline",
            &[camera_bind_group_layout, shared.texture_bind_group_layout()],
            &[TextureRectVertex::desc(), Texture3dInstanceRaw::desc()],
            include_str!("shaders/texture3d.wgsl"),
            render_tools::RenderPipelineDescriptor::default()
                .with_depth_stencil()
                .with_backface_culling(),
        );

        let vertex_buffer =
            render_tools::vertex_buffer(device, "Texture 3d", &TEXTURE_RECT_VERTICES);
        let index_buffer = render_tools::index_buffer(device, "Texture 3d", &TEXTURE_RECT_INDICES);
        let index_count = TEXTURE_RECT_INDEX_COUNT;

        //--------------------------------------------------

        let instances = HashMap::default();

        let default_texture = RawTexture::from_color(device, queue, [255, 255, 255], None, None);
        let default_texture_bind_group =
            shared.create_bind_group(device, &default_texture, Some("Default Texture"));

        let default_instances = Texture3dInstance {
            instance_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Default Texture 3d Instance Buffer"),
                size: 0,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            }),
            instance_count: 0,
        };

        //--------------------------------------------------

        Self {
            pipeline,

            vertex_buffer,
            index_buffer,
            index_count,

            instances,
            default_texture_bind_group,
            default_instances,
        }
    }

    #[deprecated]
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

    pub fn render_storage(
        &self,
        pass: &mut wgpu::RenderPass,
        camera_bind_group: &wgpu::BindGroup,
        instances: &[(Option<HandleId<Texture>>, &wgpu::Buffer, u32)],
        storage: &AssetStorage<Texture>,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);

        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        let storage = storage.get_storage();

        instances.into_iter().for_each(|instance| {
            pass.set_vertex_buffer(1, instance.1.slice(..));

            match instance.0 {
                Some(id) => {
                    let texture = storage.get(&id).unwrap();
                    pass.set_bind_group(1, texture.binding(), &[]);
                }
                None => pass.set_bind_group(1, &self.default_texture_bind_group, &[]),
            }

            pass.draw_indexed(0..self.index_count, 0, 0..instance.2);
        });
    }
}

//====================================================================

struct Texture3dInstance {
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
}

impl Texture3dInstance {
    #[inline]
    fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[Texture3dInstanceRaw],
    ) {
        render_tools::update_instance_buffer(
            device,
            queue,
            "Texture 3d",
            &mut self.instance_buffer,
            &mut self.instance_count,
            data,
        )
    }
}

//====================================================================
