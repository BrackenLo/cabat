//====================================================================

use cabat_shipyard::prelude::*;
use cabat_spatial::Transform;
use shipyard::{AllStoragesView, Component, IntoIter, SystemModificator, Unique, View};
use wgpu::util::DeviceExt;

use crate::{render_tools, Device, Queue};

//====================================================================

pub struct LightingPlugin;
impl Plugin for LightingPlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .add_workload_pre(Stages::Setup, (sys_setup_lighting).tag("lighting_setup"))
            .add_workload_last(Stages::Update, sys_prep_lighting);
    }
}

fn sys_setup_lighting(all_storages: AllStoragesView, device: Res<Device>) {
    let manager = LightingManager::new(device.inner());
    all_storages.add_unique(manager);
}

fn sys_prep_lighting(
    device: Res<Device>,
    queue: Res<Queue>,
    mut lighting: ResMut<LightingManager>,

    v_transform: View<Transform>,
    v_light: View<Light>,
) {
    let lights = (&v_transform, &v_light)
        .iter()
        .map(|(transform, light)| {
            //
            let (diffuse, specular) = match light {
                Light::Directional { diffuse, specular } => (*diffuse, *specular),
            };

            // println!("Light pos = {}", transform.translation);

            LightRaw {
                position: transform.translation.to_array(),
                direction: transform.forward().to_array(),
                diffuse,
                specular,

                padding: [0., 0.],
            }
        })
        .collect::<Vec<_>>();

    match lights.is_empty() {
        true => {
            lighting.light_array_buffer = create_default_light_buffer(device.inner());
            lighting.light_array_buffer_count = 1;
            lighting.bind_group = bind_lighting_buffers(
                device.inner(),
                &lighting.bind_group_layout,
                &lighting.global_light_buffer,
                &lighting.light_data_buffer,
                &lighting.light_array_buffer,
            );

            queue.inner().write_buffer(
                &lighting.light_data_buffer,
                0,
                bytemuck::cast_slice(&[LightData { light_count: 0 }]),
            );
        }

        false => {
            if lights.len() <= lighting.light_array_buffer_count as usize {
                queue.inner().write_buffer(
                    &lighting.light_array_buffer,
                    0,
                    bytemuck::cast_slice(lights.as_slice()),
                );

                queue.inner().write_buffer(
                    &lighting.light_data_buffer,
                    0,
                    bytemuck::cast_slice(&[LightData {
                        light_count: lights.len() as i32,
                    }]),
                );

                return;
            }

            lighting.light_array_buffer_count = lights.len() as u32;
            lighting.light_array_buffer =
                device
                    .inner()
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Light Array Buffer"),
                        contents: bytemuck::cast_slice(lights.as_slice()),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    });

            queue.inner().write_buffer(
                &lighting.light_data_buffer,
                0,
                bytemuck::cast_slice(&[LightData {
                    light_count: lights.len() as i32,
                }]),
            );
        }
    }
}

//====================================================================

#[derive(Unique)]
pub struct LightingManager {
    global_light_buffer: wgpu::Buffer,
    light_data_buffer: wgpu::Buffer,
    light_array_buffer: wgpu::Buffer,
    light_array_buffer_count: u32,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl LightingManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let global_light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Global Lighting Buffer"),
            contents: bytemuck::cast_slice(&[GlobalLightData::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Global Lighting Buffer"),
            contents: bytemuck::cast_slice(&[LightData::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_array_buffer = create_default_light_buffer(device);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light uniform bind group layout"),
            entries: &[
                render_tools::bgl_uniform_entry(0, wgpu::ShaderStages::FRAGMENT),
                render_tools::bgl_uniform_entry(1, wgpu::ShaderStages::FRAGMENT),
                render_tools::bgl_storage_entry(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });

        let bind_group = bind_lighting_buffers(
            device,
            &bind_group_layout,
            &global_light_buffer,
            &light_data_buffer,
            &light_array_buffer,
        );

        Self {
            global_light_buffer,
            light_data_buffer,
            light_array_buffer,
            light_array_buffer_count: 1,

            bind_group_layout,
            bind_group,
        }
    }

    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct GlobalLightData {
    pub ambient_color: [f32; 3],
    pub ambient_strength: f32,
}

impl Default for GlobalLightData {
    fn default() -> Self {
        Self {
            ambient_color: [1., 1., 1.],
            ambient_strength: 0.05,
        }
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug, Default)]
pub struct LightData {
    light_count: i32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct LightRaw {
    position: [f32; 3],
    direction: [f32; 3],
    diffuse: [f32; 4],
    specular: [f32; 4],

    padding: [f32; 2],
}

//====================================================================

#[derive(Component)]
pub enum Light {
    Directional {
        diffuse: [f32; 4],
        specular: [f32; 4],
    },
}

//====================================================================

fn create_default_light_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Light Array Buffer"),
        contents: bytemuck::cast_slice(&[LightRaw {
            position: [0., 0., 0.],
            direction: [0., 0., 0.],
            diffuse: [0., 0., 0., 0.],
            specular: [0., 0., 0., 0.],
            padding: [0.; 2],
        }]),
        // contents: bytemuck::cast_slice(&[LightRaw {
        //     position: [-60., 0., 0.],
        //     direction: [1., 0., 0.],
        //     diffuse: [0.3, 0.3, 0.3, 0.],
        //     specular: [1., 1., 1., 0.],
        //     padding: [0.; 2],
        // }]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    })
}

fn bind_lighting_buffers(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    global_light_buffer: &wgpu::Buffer,
    light_data_buffer: &wgpu::Buffer,
    light_array_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Light uniform bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    global_light_buffer.as_entire_buffer_binding(),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(
                    light_data_buffer.as_entire_buffer_binding(),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(
                    light_array_buffer.as_entire_buffer_binding(),
                ),
            },
        ],
    })
}

//====================================================================
