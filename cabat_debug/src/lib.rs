//====================================================================

use cabat_common::{WindowResizeEvent, WindowSize};
use cabat_renderer::{
    camera::{MainCamera, PerspectiveCamera},
    renderers::text::{Metrics, Text2dBuffer, Text2dBufferDescriptor, TextFontSystem},
    Queue,
};
use cabat_runner::tools::{Input, KeyCode, Time};
use cabat_shipyard::{prelude::*, UniqueTools};
use shipyard::{AllStoragesView, EntitiesViewMut, EntityId, Get, IntoWorkload, Unique, ViewMut};

//====================================================================

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .add_workload(Stages::Setup, sys_setup_logger)
            .add_workload_post(Stages::Update, sys_log_text);
    }
}

#[derive(Unique)]
pub struct Logger {
    text_id: EntityId,
    text: Vec<String>,
}

impl Logger {
    #[inline]
    pub fn add_log(&mut self, text: String) {
        self.text.push(text);
    }
}

fn sys_setup_logger(
    all_storages: AllStoragesView,
    mut entities: EntitiesViewMut,
    mut font_system: ResMut<TextFontSystem>,
    mut vm_text_buffer: ViewMut<Text2dBuffer>,
) {
    let text_id = entities.add_entity(
        &mut vm_text_buffer,
        Text2dBuffer::new(
            font_system.inner_mut(),
            &Text2dBufferDescriptor {
                width: Some(1920.),
                bounds_right: 1920,
                bounds_bottom: 800,
                metrics: Metrics::relative(18., 2.),
                ..Default::default()
            },
        ),
    );

    let logger = Logger {
        text_id,
        text: Vec::new(),
    };

    all_storages.add_unique(logger);
}

fn sys_log_text(
    mut logger: ResMut<Logger>,
    mut font_system: ResMut<TextFontSystem>,
    mut vm_text_buffer: ViewMut<Text2dBuffer>,
) {
    let text = logger
        .text
        .drain(..)
        .fold(String::new(), |acc, log| format!("{}{}\n", acc, log));

    let mut buffer = (&mut vm_text_buffer).get(logger.text_id).unwrap();
    buffer.set_text(font_system.inner_mut(), &text);
}

//====================================================================

pub struct FpsTrackingPlugin;

impl Plugin for FpsTrackingPlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .insert(FPSTracker::new())
            .add_workload(Stages::First, sys_tick_fps);
    }
}

#[derive(Unique)]
pub struct FPSTracker {
    second_tracker: f32,
    frame_count_this_second: u16,

    fps_list: [u16; Self::FPS_RECORD_SIZE],
    fps_instance_counter: usize,
    fps_sum: u32,
}

impl FPSTracker {
    const FPS_RECORD_SIZE: usize = 6;

    pub fn new() -> Self {
        Self {
            second_tracker: 0.,
            frame_count_this_second: 0,

            fps_list: [0; 6],
            fps_instance_counter: 0,
            fps_sum: 0,
        }
    }

    fn tick(&mut self, delta: f32, output: bool) {
        self.frame_count_this_second += 1;

        self.second_tracker += delta;

        if self.second_tracker > 1. {
            self.fps_sum -= self.fps_list[self.fps_instance_counter] as u32;
            self.fps_sum += self.frame_count_this_second as u32;
            self.fps_list[self.fps_instance_counter] = self.frame_count_this_second;
            self.fps_instance_counter = (self.fps_instance_counter + 1) % Self::FPS_RECORD_SIZE;

            self.frame_count_this_second = 0;
            self.second_tracker -= 1.;

            if output {
                let avg = self.fps_sum / Self::FPS_RECORD_SIZE as u32;
                println!("Avg fps: {}", avg);
            }
        }
    }
}

fn sys_tick_fps(mut upkeep: ResMut<FPSTracker>, time: Res<Time>) {
    upkeep.tick(time.delta_seconds(), true);
}

//====================================================================

pub struct Example3dCameraPlugin;

impl Plugin for Example3dCameraPlugin {
    fn build(self, builder: &WorkloadBuilder) {
        builder
            .insert(ExampleCamera::default())
            .add_workload(Stages::Update, (sys_update_camera, sys_move_camera))
            .add_event::<WindowResizeEvent>((sys_resize_camera).into_workload());
    }
}

#[derive(Unique, Default)]
pub struct ExampleCamera {
    pub raw: PerspectiveCamera,
}

fn sys_update_camera(
    queue: Res<Queue>,
    camera: ResMut<ExampleCamera>,
    main_camera: ResMut<MainCamera>,
) {
    if camera.is_modified() {
        main_camera.0.update_camera(queue.inner(), &camera.raw);
    }
}

fn sys_resize_camera(size: Res<WindowSize>, mut camera: ResMut<ExampleCamera>) {
    camera.raw.aspect = size.width_f32() / size.height_f32();
}

fn sys_move_camera(time: Res<Time>, keys: Res<Input<KeyCode>>, mut camera: ResMut<ExampleCamera>) {
    let left = keys.pressed(KeyCode::KeyA);
    let right = keys.pressed(KeyCode::KeyD);

    let up = keys.pressed(KeyCode::Space);
    let down = keys.pressed(KeyCode::ShiftLeft);

    let forwards = keys.pressed(KeyCode::KeyW);
    let backwards = keys.pressed(KeyCode::KeyS);

    let x_dir = (right as i8 - left as i8) as f32;
    let y_dir = (up as i8 - down as i8) as f32;
    let z_dir = (forwards as i8 - backwards as i8) as f32;

    let dir = glam::Vec3::new(x_dir, y_dir, z_dir);

    //--------------------------------------------------

    let look_left = keys.pressed(KeyCode::KeyJ);
    let look_right = keys.pressed(KeyCode::KeyL);

    let look_up = keys.pressed(KeyCode::KeyI);
    let look_down = keys.pressed(KeyCode::KeyK);

    let yaw = (look_right as i8 - look_left as i8) as f32;
    let pitch = (look_down as i8 - look_up as i8) as f32;

    //--------------------------------------------------

    let forward = camera.raw.forward() * dir.z;
    let right = camera.raw.right() * dir.x;
    let up = glam::Vec3::Y * dir.y;

    //--------------------------------------------------

    const CAMERA_MOVE_SPEED: f32 = 100.;

    camera.raw.translation += (forward + right + up) * CAMERA_MOVE_SPEED * time.delta_seconds();
    camera
        .raw
        .rotate_camera(yaw * time.delta_seconds(), pitch * time.delta_seconds());
}

//====================================================================
