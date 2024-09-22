//====================================================================

use cabat_shipyard::Plugin;

//====================================================================

pub mod common {
    pub use cabat_common::{Size, WindowResizeEvent, WindowSize};
}

pub mod renderer {
    pub use cabat_renderer::{
        camera::{Camera, CameraUniform, OrthographicCamera, PerspectiveCamera},
        render_tools, shared, text2d_pipeline, texture, texture3d_pipeliners, ClearColor, Device,
        Queue, RenderEncoder, RenderPass, RenderPassDesc, RendererPlugin, Surface, SurfaceConfig,
        Vertex,
    };
}

pub mod runner {
    pub use cabat_runner::{
        tools,
        tools::ToolsPlugin,
        window::{sys_add_window, sys_resize, Window},
        AppBuilder, AppInner, DefaultInner, Runner,
    };
}

pub mod shipyard_tools {
    pub use cabat_shipyard::{
        prelude, Event, EventHandler, Plugin, Res, ResMut, Stages, SubStages, UniqueTools,
        WorkloadBuilder, WorldTools,
    };
}

//====================================================================

pub struct DefaultPlugins;

impl Plugin for DefaultPlugins {
    fn build(
        self,
        workload_builder: cabat_shipyard::WorkloadBuilder,
    ) -> cabat_shipyard::WorkloadBuilder {
        workload_builder
            .add_plugin(runner::ToolsPlugin)
            .add_plugin(renderer::RendererPlugin)
    }
}

//====================================================================
