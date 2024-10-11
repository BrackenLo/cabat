//====================================================================

use cabat::{
    renderer::PerspectiveCamera,
    runner::{
        tools::{Input, KeyCode, Time},
        Runner,
    },
    DefaultPlugins,
};
use cabat_common::{WindowResizeEvent, WindowSize};
use cabat_renderer::{
    camera::MainCamera,
    text::{Text3dBuffer, Text3dBufferDescriptor, Text3dRenderer, TextFontSystem},
    Device, Queue,
};
use cabat_shipyard::{prelude::*, UniqueTools};
use cabat_spatial::Transform;
use glam::Vec3Swizzles;
use shipyard::{Component, EntitiesViewMut, IntoIter, IntoWorkload, Unique, ViewMut};

//====================================================================

fn main() {
    env_logger::Builder::new()
        .filter_module("cabat", log::LevelFilter::Trace)
        .filter_module("wgpu", log::LevelFilter::Warn)
        .format_timestamp(None)
        .init();

    Runner::run(|builder| {
        builder.insert(Camera::default());

        builder
            .add_plugin(DefaultPlugins)
            .add_workload(Stages::Setup, sys_setup_entities)
            .add_workload(
                Stages::Update,
                (sys_update_camera, sys_move_camera, sys_rotate_point),
            )
            .add_event::<WindowResizeEvent>((sys_resize_camera).into_workload());
    });
}

//====================================================================

#[derive(Unique, Default)]
struct Camera {
    raw: PerspectiveCamera,
}

#[derive(Component)]
struct RotatePoint {
    origin: glam::Vec3,
    distance: f32,
    progress: f32,
    speed: f32,
}

//====================================================================

fn sys_setup_entities(
    mut entities: EntitiesViewMut,
    device: Res<Device>,
    mut renderer: ResMut<Text3dRenderer>,
    mut font_system: ResMut<TextFontSystem>,

    mut vm_pos: ViewMut<Transform>,
    mut vm_text_buffers: ViewMut<Text3dBuffer>,
    mut vm_rotate_point: ViewMut<RotatePoint>,
) {
    entities.add_entity(
        (&mut vm_pos, &mut vm_text_buffers, &mut vm_rotate_point),
        (
            Transform::from_translation(glam::Vec3::ZERO),
            Text3dBuffer::new(
                device.inner(),
                &mut renderer,
                font_system.inner_mut(),
                &Text3dBufferDescriptor {
                    text: "Hello World! 12345 \nABCDE",
                    // text: "A\nB",
                    // width: todo!(),
                    pos: glam::vec3(0., 0., 20.),
                    ..Default::default()
                },
            ),
            RotatePoint {
                origin: glam::Vec3::ZERO,
                distance: 100.,
                progress: 0.,
                speed: 1.,
            },
        ),
    );
}

//====================================================================

fn sys_rotate_point(
    time: Res<Time>,
    mut vm_transform: ViewMut<Transform>,
    mut vm_rotate_point: ViewMut<RotatePoint>,
) {
    (&mut vm_transform, &mut vm_rotate_point)
        .iter()
        .for_each(|(mut transform, rotation)| {
            rotation.progress += time.delta_seconds() * rotation.speed;

            transform.translation.x =
                rotation.progress.sin() * rotation.distance + rotation.origin.x;

            transform.translation.z =
                rotation.progress.cos() * rotation.distance + rotation.origin.z;

            let pos_2d = transform.translation.xz();
            let target_2d = rotation.origin.xz();

            let val = pos_2d - target_2d;
            let angle = f32::atan2(val.y, val.x);

            let angle_quat = glam::Quat::from_rotation_y(angle - 90_f32.to_radians());

            transform.rotation = angle_quat.inverse();
        });
}

//====================================================================

fn sys_update_camera(queue: Res<Queue>, camera: ResMut<Camera>, main_camera: ResMut<MainCamera>) {
    if camera.is_modified() {
        main_camera.0.update_camera(queue.inner(), &camera.raw);
    }
}

fn sys_resize_camera(size: Res<WindowSize>, mut camera: ResMut<Camera>) {
    camera.raw.aspect = size.width_f32() / size.height_f32();
}

fn sys_move_camera(time: Res<Time>, keys: Res<Input<KeyCode>>, mut camera: ResMut<Camera>) {
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
