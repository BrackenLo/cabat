//====================================================================

use std::num::NonZeroU32;

use wgpu::util::DeviceExt;

use crate::{texture::RawTexture, Vertex};

//====================================================================

pub struct RenderPipelineDescriptor<'a> {
    pub primitive: wgpu::PrimitiveState,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub multisample: wgpu::MultisampleState,
    pub fragment_targets: Option<&'a [Option<wgpu::ColorTargetState>]>,
    pub multiview: Option<NonZeroU32>,
    pub cache: Option<&'a wgpu::PipelineCache>,
}

impl<'a> Default for RenderPipelineDescriptor<'a> {
    fn default() -> Self {
        Self {
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment_targets: None,
            multiview: None,
            cache: None,
        }
    }
}

impl RenderPipelineDescriptor<'_> {
    pub fn with_depth_stencil(mut self) -> Self {
        self.depth_stencil = Some(wgpu::DepthStencilState {
            format: RawTexture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        self
    }

    pub fn with_backface_culling(mut self) -> Self {
        self.primitive.cull_mode = Some(wgpu::Face::Back);
        self
    }
}

pub fn create_pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    label: &str,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    vertex_buffers: &[wgpu::VertexBufferLayout],
    shader_module_data: &str,

    desc: RenderPipelineDescriptor,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{} layout", label)),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{} shader module", label)),
        source: wgpu::ShaderSource::Wgsl(shader_module_data.into()),
    });

    let default_fragment_targets = [Some(wgpu::ColorTargetState {
        format: config.format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::all(),
    })];
    let fragment_targets = desc.fragment_targets.unwrap_or(&default_fragment_targets);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            compilation_options: Default::default(),
            buffers: vertex_buffers,
        },
        primitive: desc.primitive,
        depth_stencil: desc.depth_stencil,
        multisample: desc.multisample,
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            compilation_options: Default::default(),
            targets: fragment_targets,
        }),
        multiview: desc.multiview,
        cache: desc.cache,
    })
}

//====================================================================

/// bind group layout uniform entry
pub fn bgl_uniform_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub fn bgl_texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

pub fn bgl_sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

pub fn vertex_buffer<T: Vertex>(device: &wgpu::Device, label: &str, data: &[T]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Vertex Buffer", label)),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub fn index_buffer(device: &wgpu::Device, label: &str, data: &[u16]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Index Buffer", label)),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::INDEX,
    })
}

//====================================================================

pub fn update_instance_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,

    label: &str,
    buffer: &mut wgpu::Buffer,
    instance_count: &mut u32,

    data: &[T],
) {
    if data.len() == 0 {
        // Nothing to update
        if *instance_count != 0 {
            // Empty buffer and reset instance count
            *buffer = create_instance_buffer(device, label, data);
            *instance_count = 0;
        }

        return;
    }

    // We can fit all data inside existing buffer
    if data.len() <= *instance_count as usize {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
        *instance_count = data.len() as u32; // TODO - add additional variable for buffer size
        return;
    }

    // Buffer is too small to fit new data. Create a new bigger one.
    *instance_count = data.len() as u32;
    *buffer = create_instance_buffer(device, label, data);
}

pub fn create_instance_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &str,
    data: &[T],
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Instance Buffer", label)),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    })
}

//====================================================================
