//====================================================================

use cabat_shipyard::Plugin;

//====================================================================

pub mod common {
    pub use cabat_common::{Size, WindowResizeEvent, WindowSize};
}

pub mod renderer {
    pub use cabat_renderer::{
        camera::{Camera, CameraUniform, OrthographicCamera, PerspectiveCamera},
        crates, plugins, render_tools, shared, text, texture, texture3d_pipeliners, ClearColor,
        Device, FullRendererPlugin, Queue, RenderEncoder, RenderPass, RenderPassDesc, Surface,
        SurfaceConfig, Vertex,
    };
}

pub mod runner {
    pub use cabat_runner::{
        tools,
        tools::ToolsPlugin,
        window::{sys_add_window, sys_resize, Window},
        Runner,
    };
}

pub mod shipyard_tools {
    pub use cabat_shipyard::{
        prelude, Event, EventHandler, Plugin, Res, ResMut, Stages, SubStages, UniqueTools,
        WorkloadBuilder, WorldTools,
    };
}

pub mod spatial {
    pub use cabat_spatial::Transform;
}

pub mod assets {
    pub use cabat_assets::{
        asset_loader::AssetTypeLoader,
        asset_storage::AssetStorage,
        handle::{Handle, HandleId, HandleInner},
        Asset, AssetStoragePlugin,
    };
}

//====================================================================

pub struct DefaultPlugins;

impl Plugin for DefaultPlugins {
    fn build(self, workload_builder: &cabat_shipyard::WorkloadBuilder) {
        workload_builder
            .add_plugin(runner::ToolsPlugin)
            .add_plugin(assets::AssetStoragePlugin)
            .add_plugin(renderer::FullRendererPlugin);
    }
}

//====================================================================
