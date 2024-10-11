//====================================================================

use cabat::{assets::AssetStorage, DefaultPlugins};
use cabat_renderer::texture3d_renderer::Sprite;
use cabat_runner::{tools::Time, Runner};
use cabat_shipyard::{Res, ResMut, Stages};
use cabat_spatial::Transform;
use shipyard::{AllStoragesView, Component, EntitiesViewMut, IntoIter, ViewMut};

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
            .add_workload(Stages::Setup, sys_spawn_entities)
            .add_workload(Stages::Update, sys_spin);
    });
}

//====================================================================

#[derive(Component, Default)]
struct Spin {
    progress: f32,
}

//====================================================================

fn sys_spawn_entities(
    all_storages: AllStoragesView,
    mut assets: ResMut<AssetStorage>,

    mut entities: EntitiesViewMut,
    mut vm_sprites: ViewMut<Sprite>,
    mut vm_transform: ViewMut<Transform>,
    mut vm_spin: ViewMut<Spin>,
) {
    let handle = assets.load_file(all_storages, "yay.jpg").unwrap();

    entities.add_entity(
        (&mut vm_sprites, &mut vm_transform, &mut vm_spin),
        (
            Sprite {
                texture: None,
                width: 40.,
                height: 40.,
                color: [1., 0., 0., 1.],
            },
            Transform::from_translation(glam::Vec3::new(0., 0., 50.)),
            Spin::default(),
        ),
    );

    entities.add_entity(
        (&mut vm_sprites, &mut vm_transform, &mut vm_spin),
        (
            Sprite {
                texture: Some(handle),
                width: 20.,
                height: 20.,
                color: [1., 1., 1., 1.],
            },
            Transform::from_translation(glam::Vec3::new(0., 0., 35.)),
            Spin { progress: 0.6 },
        ),
    );
}

fn sys_spin(time: Res<Time>, mut vm_transform: ViewMut<Transform>, mut vm_spin: ViewMut<Spin>) {
    (&mut vm_transform, &mut vm_spin)
        .iter()
        .for_each(|(mut transform, spin)| {
            spin.progress += time.delta_seconds();

            transform.rotation = glam::Quat::from_rotation_y(spin.progress.sin());
        });
}

//====================================================================
