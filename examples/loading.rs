//====================================================================

use std::path::PathBuf;

use cabat::{
    assets::{AssetLoader, AssetStorage},
    DefaultPlugins,
};
use cabat_assets::{Asset, RegisterAssetLoader};
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
            .register_loader(TextLoader)
            .add_workload(Stages::Setup, sys_load_stuff);
    });
}

fn sys_load_stuff(storages: AllStoragesView, asset_storage: Res<AssetStorage<MyString>>) {
    let path = PathBuf::new().join("ipsum.txt");

    let data = asset_storage.load_file(storages, &path).unwrap();

    println!("{:?}", data);
}

//====================================================================

#[derive(Debug)]
pub struct MyString(pub String);

impl Asset for MyString {}

pub struct TextLoader;

impl AssetLoader<MyString> for TextLoader {
    fn load_path(
        &self,
        _all_storages: AllStoragesView,
        path: &std::path::Path,
    ) -> cabat_assets::Result<MyString> {
        Ok(MyString(std::fs::read_to_string(path)?))
    }

    fn extensions(&self) -> &[&str] {
        &["txt"]
    }
}

//====================================================================
