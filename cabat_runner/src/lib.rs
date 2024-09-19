//====================================================================

use std::{marker::PhantomData, sync::Arc, time::Duration};

use shipyard::World;
use shipyard_shared::Size;
use shipyard_tools::{Stages, WorkloadBuilder};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{WindowAttributes, WindowId},
};

pub mod window;

//====================================================================

pub trait AppBuilder {
    fn build(builder: WorkloadBuilder) -> WorkloadBuilder;
}

pub trait AppInner {
    fn new(event_loop: &ActiveEventLoop) -> Self;
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    );

    fn resumed(&mut self);
}

//====================================================================

pub struct Runner<Builder: AppBuilder> {
    inner: Option<DefaultInner<Builder>>,
}

impl<Builder: AppBuilder> Runner<Builder> {
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn run(mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut self).unwrap();
    }
}

//--------------------------------------------------

impl<Builder: AppBuilder> ApplicationHandler for Runner<Builder> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::trace!("App Resumed - Creating inner app");

        self.inner = Some(DefaultInner::new(event_loop));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(inner) = &mut self.inner {
            inner.window_event(event_loop, window_id, event);
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let Some(inner) = &mut self.inner {
            match cause {
                StartCause::ResumeTimeReached { .. } => inner.resumed(),
                // StartCause::WaitCancelled { start, requested_resume } => todo!(),
                // StartCause::Poll => todo!(),
                // StartCause::Init => todo!(),
                _ => {}
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }
}

//====================================================================

const TIMESTEP: f32 = 1. / 75.;

pub struct DefaultInner<Builder: AppBuilder> {
    phantom: PhantomData<Builder>,
    world: World,
    timestep: Duration,
}

impl<Builder: AppBuilder> DefaultInner<Builder> {
    fn resize(&mut self, new_size: Size<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            log::warn!("Resize width or height of '0' provided");
            return;
        }

        self.world.run_with_data(window::sys_resize, new_size);
    }

    fn tick(&mut self) {
        self.world.run_workload(Stages::First).unwrap();

        shipyard_tools::activate_events(&self.world);

        // TODO
        // self.world.run_workload(Stages::FixedUpdate).unwrap();

        self.world.run_workload(Stages::Update).unwrap();
        self.world.run_workload(Stages::Render).unwrap();
        self.world.run_workload(Stages::Last).unwrap();
    }
}

impl<Builder: AppBuilder> AppInner for DefaultInner<Builder> {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let world = shipyard::World::new();

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        world.run_with_data(window::sys_add_window, window);

        Builder::build(WorkloadBuilder::new(&world)).build();

        world.run_workload(Stages::Setup).unwrap();

        Self {
            phantom: PhantomData,
            world,
            timestep: Duration::from_secs_f32(TIMESTEP),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                self.resize(Size::new(new_size.width, new_size.height))
            }

            WindowEvent::Destroyed => log::error!("Window was destroyed"), // panic!("Window was destroyed"),
            WindowEvent::CloseRequested => {
                log::info!("Close requested. Closing App.");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.tick();

                event_loop.set_control_flow(ControlFlow::wait_duration(self.timestep));
            }

            // WindowEvent::KeyboardInput { event, .. } => {
            //     if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
            //         self.world.run_with_data(
            //             tools::sys_process_input::<winit::keyboard::KeyCode>,
            //             (key, event.state.is_pressed()),
            //         );
            //     }
            // }

            // WindowEvent::MouseInput { state, button, .. } => self.world.run_with_data(
            //     tools::sys_process_input::<winit::event::MouseButton>,
            //     (button, state.is_pressed()),
            // ),

            // WindowEvent::CursorMoved { position, .. } => self.world.run_with_data(
            //     tools::sys_process_mouse_pos,
            //     [position.x as f32, position.y as f32],
            // ),
            // WindowEvent::MouseWheel { delta, .. } => match delta {
            //     winit::event::MouseScrollDelta::LineDelta(h, v) => {
            //         self.world.run_with_data(tools::sys_process_wheel, [h, v])
            //     }
            //     winit::event::MouseScrollDelta::PixelDelta(_) => {}
            // },
            _ => {}
        }
    }

    fn resumed(&mut self) {
        self.world
            .run(|window: shipyard::UniqueView<window::Window>| window.request_redraw());
    }
}

//====================================================================
