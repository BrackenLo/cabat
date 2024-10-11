//====================================================================

use std::path::PathBuf;

use cabat::{assets::AssetStorage, DefaultPlugins};
use cabat_assets::Asset;
use cabat_runner::Runner;
use cabat_shipyard::prelude::*;
use shipyard::AllStoragesView;

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
            .add_workload(Stages::Setup, sys_load_stuff);
    });
}

fn sys_load_stuff(storages: AllStoragesView, mut asset_storage: ResMut<AssetStorage>) {
    let path = PathBuf::new().join("hello.txt");

    let data = asset_storage.load_file::<String>(storages, &path).unwrap();

    println!("data = {}", data);
}

//====================================================================

pub struct ThisStruct;

impl Asset for ThisStruct {}
