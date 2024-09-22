//====================================================================

use wgpu::util::DeviceExt;

//====================================================================

pub struct Camera {
    camera_buffer: wgpu::Buffer,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group: wgpu::BindGroup,
}

impl Camera {
    pub fn new<C: CameraUniform>(device: &wgpu::Device, camera: &C) -> Self {
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: bytemuck::cast_slice(&[camera.into_uniform()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(camera_buffer.as_entire_buffer_binding()),
            }],
        });

        Self {
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
        }
    }

    #[inline]
    pub fn update_camera<C: CameraUniform>(&self, queue: &wgpu::Queue, camera: &C) {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera.into_uniform()]),
        );
    }

    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }
}

//====================================================================

pub trait CameraUniform {
    fn into_uniform(&self) -> CameraUniformRaw;
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct CameraUniformRaw {
    view_projection: [f32; 16],
    camera_position: [f32; 3],
    _padding: u32,
}
impl CameraUniformRaw {
    pub fn new(view_projection: [f32; 16], camera_position: [f32; 3]) -> Self {
        Self {
            view_projection,
            camera_position,
            _padding: 0,
        }
    }
}

//--------------------------------------------------

#[derive(Debug, Clone)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub z_near: f32,
    pub z_far: f32,

    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        Self {
            left: 0.,
            right: 1920.,
            bottom: 0.,
            top: 1080.,
            z_near: 0.,
            z_far: 1000000.,

            translation: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
        }
    }
}

impl CameraUniform for OrthographicCamera {
    fn into_uniform(&self) -> CameraUniformRaw {
        CameraUniformRaw::new(self.get_projection(), self.translation.into())
    }
}

impl OrthographicCamera {
    fn get_projection(&self) -> [f32; 16] {
        let projection_matrix = glam::Mat4::orthographic_lh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.z_near,
            self.z_far,
        );

        // BUG  - find out why camera axis is wrong way around
        let transform_matrix =
            glam::Mat4::from_rotation_translation(self.rotation, -self.translation);

        (projection_matrix * transform_matrix).to_cols_array()
    }

    pub fn new_sized(width: f32, height: f32) -> Self {
        Self {
            left: 0.,
            right: width,
            bottom: 0.,
            top: height,
            ..Default::default()
        }
    }

    pub fn _new_centered(half_width: f32, half_height: f32) -> Self {
        Self {
            left: -half_width,
            right: half_width,
            bottom: -half_height,
            top: half_height,
            ..Default::default()
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        let half_width = width / 2.;
        let half_height = height / 2.;

        self.left = -half_width;
        self.right = half_width;
        self.top = half_height;
        self.bottom = -half_height;
    }

    pub fn screen_to_camera(&self, screen_pos: glam::Vec2) -> glam::Vec2 {
        // TODO/FIX - Test this function with different ratios
        screen_pos + self.translation.truncate()
            - glam::vec2((self.right - self.left) / 2., (self.top - self.bottom) / 2.)
    }
}

//--------------------------------------------------

#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    pub up: glam::Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub z_near: f32,
    pub z_far: f32,

    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            up: glam::Vec3::Y,
            aspect: 1.7777777778,
            fovy: 45.,
            z_near: 0.1,
            z_far: 1000000.,

            translation: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
        }
    }
}

impl CameraUniform for PerspectiveCamera {
    fn into_uniform(&self) -> CameraUniformRaw {
        CameraUniformRaw::new(self.get_projection(), self.translation.into())
    }
}

impl PerspectiveCamera {
    fn get_projection(&self) -> [f32; 16] {
        let forward = (self.rotation * glam::Vec3::Z).normalize();

        let projection_matrix =
            glam::Mat4::perspective_lh(self.fovy, self.aspect, self.z_near, self.z_far);

        let view_matrix =
            glam::Mat4::look_at_lh(self.translation, self.translation + forward, self.up);

        (projection_matrix * view_matrix).to_cols_array()
    }

    pub fn forward(&self) -> glam::Vec3 {
        let (x, _, z) = (self.rotation * glam::Vec3::Z).into();
        glam::Vec3::new(x, 0., z).normalize()
    }

    pub fn right(&self) -> glam::Vec3 {
        let (x, _, z) = (self.rotation * glam::Vec3::X).into();
        glam::Vec3::new(x, 0., z).normalize()
    }

    pub fn rotate_camera(&mut self, yaw: f32, pitch: f32) {
        let yaw_rotation = glam::Quat::from_rotation_y(yaw);
        let pitch_rotation = glam::Quat::from_rotation_x(pitch);

        self.rotation = yaw_rotation * self.rotation * pitch_rotation;
    }
}

//====================================================================
