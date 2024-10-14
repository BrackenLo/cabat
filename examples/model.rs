//====================================================================

use cabat::{renderer::PerspectiveCamera, DefaultPlugins};
use cabat_common::{WindowResizeEvent, WindowSize};
use cabat_renderer::{
    camera::MainCamera, renderers::model::Model, shared::DefaultRendererAssets, Queue,
};
use cabat_runner::{
    tools::{Input, KeyCode, Time},
    Runner,
};
use cabat_shipyard::{prelude::*, UniqueTools};
use cabat_spatial::Transform;
use shipyard::{
    AllStoragesView, Component, EntitiesViewMut, IntoIter, IntoWorkload, Unique, ViewMut,
};

//====================================================================

fn main() {
    env_logger::Builder::new()
        .filter_module("cabat", log::LevelFilter::Trace)
        .filter_module("wgpu", log::LevelFilter::Warn)
        .format_timestamp(None)
        .init();

    Runner::run(|builder| {
        builder
            .insert(Camera::default())
            .add_plugin(DefaultPlugins)
            .add_workload(Stages::Setup, sys_spawn_entities)
            .add_workload(
                Stages::Update,
                (sys_spin, sys_update_camera, sys_move_camera),
            )
            .add_event::<WindowResizeEvent>((sys_resize_camera).into_workload());
    });
}

//====================================================================

#[derive(Unique, Default)]
struct Camera {
    raw: PerspectiveCamera,
}

#[derive(Component, Default)]
struct Spin {
    progress: f32,
    speed: f32,
}

//====================================================================

fn sys_spawn_entities(
    all_storages: AllStoragesView,
    mut default_assets: ResMut<DefaultRendererAssets>,

    mut entities: EntitiesViewMut,
    mut vm_sprites: ViewMut<Model>,
    mut vm_transform: ViewMut<Transform>,
    mut vm_spin: ViewMut<Spin>,
) {
    default_assets.load_cube(&all_storages);
    let default_handle = default_assets.get_cube().unwrap();

    entities.add_entity(
        (&mut vm_sprites, &mut vm_transform, &mut vm_spin),
        (
            Model {
                data: default_handle,
                color: [1., 0., 0.5, 1.],
            },
            // Transform::from_translation(glam::Vec3::new(0., 0., 50.)),
            Transform::from_translation_scale(
                glam::Vec3::new(0., 0., 50.),
                glam::Vec3::new(30., 30., 30.),
            ),
            Spin::default(),
        ),
    );
}

fn sys_spin(time: Res<Time>, mut vm_transform: ViewMut<Transform>, mut vm_spin: ViewMut<Spin>) {
    (&mut vm_transform, &mut vm_spin)
        .iter()
        .for_each(|(mut transform, spin)| {
            spin.progress += time.delta_seconds() * spin.speed;

            transform.rotation = glam::Quat::from_rotation_y(spin.progress.sin());
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
