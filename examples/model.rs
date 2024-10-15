//====================================================================

use cabat::DefaultPlugins;
use cabat_debug::{Example3dCameraPlugin, ExampleCamera, Logger, LoggingPlugin};
use cabat_renderer::{lighting::Light, renderers::model::Model, shared::DefaultRendererAssets};
use cabat_runner::{tools::Time, Runner};
use cabat_shipyard::prelude::*;
use cabat_spatial::Transform;
use shipyard::{AllStoragesView, Component, EntitiesViewMut, IntoIter, View, ViewMut};

//====================================================================

fn main() {
    env_logger::Builder::new()
        .filter_module("cabat", log::LevelFilter::Trace)
        .filter_module("wgpu", log::LevelFilter::Warn)
        .format_timestamp(None)
        .init();

    Runner::run(|builder| {
        builder
            .add_plugin(DefaultPlugins)
            .add_plugin(Example3dCameraPlugin)
            .add_plugin(LoggingPlugin)
            .add_workload(Stages::Setup, sys_spawn_entities)
            .add_workload(Stages::Update, sys_spin)
            .add_workload_post(Stages::Update, sys_log_stuff);
    });
}

//====================================================================

#[derive(Component)]
struct Spin {
    progress: f32,
    speed: f32,
    origin: glam::Vec3,
    distance: f32,
}

impl Default for Spin {
    fn default() -> Self {
        Self {
            progress: 0.,
            speed: 1.,
            origin: glam::Vec3::new(0., 0., 50.),
            distance: 100.,
        }
    }
}

//====================================================================

fn sys_spawn_entities(
    all_storages: AllStoragesView,
    mut default_assets: ResMut<DefaultRendererAssets>,

    mut entities: EntitiesViewMut,
    mut vm_model: ViewMut<Model>,
    mut vm_transform: ViewMut<Transform>,
    mut vm_spin: ViewMut<Spin>,
    mut vm_light: ViewMut<Light>,
) {
    default_assets.load_cube(&all_storages);
    let default_handle = default_assets.get_cube().unwrap();

    entities.add_entity(
        (&mut vm_model, &mut vm_transform),
        (
            Model {
                data: default_handle.clone(),
                color: [1., 0., 0.5, 1.],
            },
            Transform::from_translation_rotatation_scale(
                glam::Vec3::new(0., 0., 50.),
                glam::Quat::IDENTITY,
                glam::Vec3::splat(30.),
            ),
        ),
    );

    entities.add_entity(
        (
            &mut vm_model,
            &mut vm_transform,
            &mut vm_light,
            &mut vm_spin,
        ),
        (
            Model {
                data: default_handle,
                color: [1., 1., 1., 1.],
            },
            Transform::from_translation_scale(
                glam::Vec3::new(0., 0., 0.),
                glam::Vec3::new(5., 5., 5.),
            ),
            Light::Directional {
                diffuse: [0.3, 0.3, 0.3, 0.],
                specular: [1., 1., 1., 0.],
            },
            Spin::default(),
        ),
    );
}

fn sys_spin(time: Res<Time>, mut vm_transform: ViewMut<Transform>, mut vm_spin: ViewMut<Spin>) {
    (&mut vm_transform, &mut vm_spin)
        .iter()
        .for_each(|(mut transform, spin)| {
            spin.progress += time.delta_seconds() * spin.speed;

            // if spin.progress.sin() > 0.98 {
            //     println!("Full Spin");
            // }

            transform.translation.x = spin.progress.sin() * spin.distance + spin.origin.x;
            transform.translation.z = spin.progress.cos() * spin.distance + spin.origin.z;
        });
}

fn sys_log_stuff(
    mut logger: ResMut<Logger>,
    camera: Res<ExampleCamera>,
    v_transform: View<Transform>,
    v_light: View<Light>,
) {
    logger.add_log(format!(
        "Camera Translation = {:.2}",
        camera.raw.translation
    ));

    let light_translation = (&v_transform, &v_light)
        .iter()
        .next()
        .unwrap()
        .0
        .translation;

    logger.add_log(format!("Light Translation = {:.2}", light_translation))
}

//====================================================================
