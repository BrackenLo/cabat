//====================================================================

use cabat::{
    renderer::{Camera, PerspectiveCamera},
    runner::{
        tools::{Input, KeyCode, Time},
        Runner,
    },
    DefaultPlugins,
};
use cabat_common::{WindowResizeEvent, WindowSize};
use cabat_renderer::{
    text::{Text3dBuffer, Text3dRenderer},
    Device, Queue, RenderPass, SurfaceConfig,
};
use cabat_shipyard::prelude::*;
use shipyard::{
    AllStoragesView, Component, EntitiesViewMut, IntoIter, IntoWorkload, Unique, View, ViewMut,
};

//====================================================================

fn main() {
    env_logger::Builder::new()
        .filter_module("cabat", log::LevelFilter::Trace)
        .init();

    Runner::run(|builder| builder.add_plugin(DefaultPlugins).add_plugin(Text3dPlugin));
}

//====================================================================

#[derive(Unique)]
struct MainCamera {
    camera: Camera,
    raw: PerspectiveCamera,
}

#[derive(Component)]
struct Translation {
    pos: glam::Vec3,
}

//====================================================================

pub struct Text3dPlugin;
impl Plugin for Text3dPlugin {
    fn build(self, workload_builder: WorkloadBuilder) -> WorkloadBuilder {
        workload_builder
            .add_workload(
                Stages::Setup,
                (sys_setup_renderer, sys_setup_entities).into_sequential_workload(),
            )
            .add_workload(
                Stages::Update,
                (sys_update_camera, sys_move_camera).into_workload(),
            )
            .add_workload_post(Stages::Update, (sys_update_text).into_workload())
            .add_workload(Stages::Render, (sys_render).into_workload())
            .add_event::<WindowResizeEvent>((sys_resize_camera).into_workload())
    }
}

fn sys_setup_renderer(
    all_storages: AllStoragesView,
    device: Res<Device>,
    config: Res<SurfaceConfig>,
) {
    let raw = PerspectiveCamera::default();
    let camera = MainCamera {
        camera: Camera::new(device.inner(), &raw),
        raw,
    };

    let renderer = Text3dRenderer::new(
        device.inner(),
        config.inner(),
        camera.camera.bind_group_layout(),
    );
    all_storages.add_unique(renderer);
    all_storages.add_unique(camera);
}

fn sys_setup_entities(
    mut entities: EntitiesViewMut,
    device: Res<Device>,
    mut renderer: ResMut<Text3dRenderer>,

    mut vm_pos: ViewMut<Translation>,
    mut vm_text_buffers: ViewMut<Text3dBuffer>,
) {
    entities.add_entity(
        (&mut vm_pos, &mut vm_text_buffers),
        (
            Translation {
                pos: glam::Vec3::ZERO,
            },
            Text3dBuffer::new(device.inner(), &mut renderer),
        ),
    );
}

fn sys_update_text(
    device: Res<Device>,
    queue: Res<Queue>,

    mut renderer: ResMut<Text3dRenderer>,
    // v_pos: View<Translation>,
    mut vm_text_buffers: ViewMut<Text3dBuffer>,
) {
    renderer.prep(device.inner(), queue.inner(), (&mut vm_text_buffers).iter());
}

fn sys_render(
    mut render_pass: ResMut<RenderPass>,
    renderer: Res<Text3dRenderer>,
    v_text_buffers: View<Text3dBuffer>,

    camera: Res<MainCamera>,
) {
    renderer.render(
        render_pass.pass(),
        camera.camera.bind_group(),
        v_text_buffers.iter(),
    );
}

//====================================================================

fn sys_update_camera(queue: Res<Queue>, camera: ResMut<MainCamera>) {
    if camera.is_modified() {
        camera.camera.update_camera(queue.inner(), &camera.raw);
    }
}

fn sys_resize_camera(size: Res<WindowSize>, mut camera: ResMut<MainCamera>) {
    camera.raw.aspect = size.width_f32() / size.height_f32();
}

fn sys_move_camera(time: Res<Time>, keys: Res<Input<KeyCode>>, mut camera: ResMut<MainCamera>) {
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
