//====================================================================

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};

use cabat_assets::{
    asset_storage::AssetStorage,
    handle::{Handle, HandleId},
    Asset,
};
use cabat_shipyard::prelude::*;
use cabat_spatial::Transform;
use rustc_hash::FxHasher;
use shipyard::{AllStoragesView, Component, IntoIter, SystemModificator, Unique, View};

use crate::{
    camera::MainCamera,
    lighting::{LightingManager, LightingPlugin},
    render_tools,
    shared::SharedRendererResources,
    texture::Texture,
    Device, Queue, RenderPass, SurfaceConfig, Vertex,
};

//====================================================================

pub struct ModelPlugin;

impl Plugin for ModelPlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .add_plugin(LightingPlugin)
            .add_workload_pre(
                Stages::Setup,
                sys_setup_renderer.after_all("lighting_setup"),
            )
            .add_workload_last(Stages::Update, sys_prep_models)
            .add_workload(Stages::Render, sys_render_models);
    }
}

fn sys_setup_renderer(
    all_storages: AllStoragesView,
    device: Res<Device>,
    config: Res<SurfaceConfig>,
    shared: Res<SharedRendererResources>,
    camera: Res<MainCamera>,
    lighting: Res<LightingManager>,
) {
    let renderer = ModelRenderer::new(
        device.inner(),
        config.inner(),
        &shared,
        camera.bind_group_layout(),
        lighting.bind_group_layout(),
    );

    all_storages.add_unique(renderer);
}

fn sys_prep_models(
    device: Res<Device>,
    queue: Res<Queue>,
    mut renderer: ResMut<ModelRenderer>,
    v_model: View<Model>,
    v_transform: View<Transform>,
) {
    let instances =
        (&v_transform, &v_model)
            .iter()
            .fold(HashMap::new(), |mut acc, (transform, model)| {
                let instance = ModelInstanceRaw {
                    transform: transform.to_array(),
                    color: model.color,
                    normal: transform.to_normal_matrix_array(),
                };

                acc.entry(model.data.id())
                    .or_insert(Vec::new())
                    .push(instance);

                acc
            });

    let mut previous = renderer
        .instances
        .keys()
        .map(|id| *id)
        .collect::<HashSet<_>>();

    instances.into_iter().for_each(|(id, raw)| {
        previous.remove(&id);

        renderer
            .instances
            .entry(id)
            .and_modify(|instance| instance.update(device.inner(), queue.inner(), raw.as_slice()))
            .or_insert(ModelInstance {
                instance_buffer: render_tools::create_instance_buffer(
                    device.inner(),
                    "Model",
                    raw.as_slice(),
                ),
                instance_count: raw.len() as u32,
            });
    });

    // TEST
    previous.into_iter().for_each(|to_remove| {
        renderer.instances.remove(&to_remove);
    })
}

fn sys_render_models(
    mut pass: ResMut<RenderPass>,
    renderer: Res<ModelRenderer>,
    camera: Res<MainCamera>,

    model_storage: Res<AssetStorage<ModelData>>,
    texture_storage: Res<AssetStorage<Texture>>,
    lighting: Res<LightingManager>,
) {
    let instances = renderer
        .instances
        .iter()
        .map(|(id, instance)| (*id, instance))
        .collect::<Vec<_>>();

    renderer.render_storage(
        pass.pass(),
        camera.bind_group(),
        lighting.bind_group(),
        instances.as_slice(),
        &model_storage,
        &texture_storage,
    );
}

//====================================================================

#[derive(Component)]
pub struct Model {
    pub data: Handle<ModelData>,
    pub color: [f32; 4],
}

//====================================================================

pub struct ModelData {
    pub meshes: Vec<Mesh>,
    // pub materials: Vec<Material>,
}

impl Asset for ModelData {}

//--------------------------------------------------

// pub struct Material {
//     name: String,
//     diffuse_texture: Handle<Texture>,
// }

//--------------------------------------------------

pub struct Mesh {
    // name: String,
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) material: Handle<Texture>,
}

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Float32x3
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//--------------------------------------------------

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct ModelInstanceRaw {
    pub transform: [f32; 16],
    pub color: [f32; 4],
    pub normal: [f32; 9],
}

impl Vertex for ModelInstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
            // Transform Matrix
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,

            // Color
            7 => Float32x4,

            // Normal Matrix
            8 => Float32x3,
            9 => Float32x3,
            10 => Float32x3,


        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelInstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//====================================================================

#[derive(Unique)]
pub struct ModelRenderer {
    pipeline: wgpu::RenderPipeline,

    instances: HashMap<HandleId<ModelData>, ModelInstance, BuildHasherDefault<FxHasher>>,
}

impl ModelRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shared: &SharedRendererResources,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        lighting_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let pipeline = render_tools::create_pipeline(
            device,
            config,
            "Model Pipeline",
            &[
                camera_bind_group_layout,
                lighting_bind_group_layout,
                shared.texture_bind_group_layout(),
            ],
            &[ModelVertex::desc(), ModelInstanceRaw::desc()],
            include_str!("shaders/model.wgsl"),
            render_tools::RenderPipelineDescriptor::default()
                .with_depth_stencil()
                .with_backface_culling(),
        );

        Self {
            pipeline,
            instances: HashMap::default(),
        }
    }

    pub fn render_storage(
        &self,
        pass: &mut wgpu::RenderPass,
        camera_bind_group: &wgpu::BindGroup,
        lighting_bind_group: &wgpu::BindGroup,
        instances: &[(HandleId<ModelData>, &ModelInstance)],
        model_storage: &AssetStorage<ModelData>,
        texture_storage: &AssetStorage<Texture>,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_bind_group(1, lighting_bind_group, &[]);

        let model_storage = model_storage.get_storage();
        let texture_storage = texture_storage.get_storage();

        instances.into_iter().for_each(|(handle, instance)| {
            let model = model_storage.get(handle).unwrap();

            pass.set_vertex_buffer(1, instance.instance_buffer.slice(..));

            model.meshes.iter().for_each(|mesh| {
                let texture = texture_storage.get(&mesh.material.id()).unwrap();

                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_bind_group(2, texture.binding(), &[]);

                pass.draw_indexed(0..mesh.index_count, 0, 0..instance.instance_count);
            });
        });
    }
}

//====================================================================

pub struct ModelInstance {
    pub instance_buffer: wgpu::Buffer,
    pub instance_count: u32,
}

impl ModelInstance {
    #[inline]
    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[ModelInstanceRaw]) {
        render_tools::update_instance_buffer(
            device,
            queue,
            "Model",
            &mut self.instance_buffer,
            &mut self.instance_count,
            data,
        )
    }
}

//====================================================================
