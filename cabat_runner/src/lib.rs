//====================================================================

use std::{sync::Arc, time::Duration};

use cabat_common::Size;
use cabat_shipyard::{Stages, WorkloadBuilder};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{WindowAttributes, WindowId},
};

pub mod tools;
pub mod window;

//====================================================================

enum RunnerState {
    Waiting(shipyard::World),
    Running(RunnerInner),
}

pub struct Runner {
    state: RunnerState,
}

impl Runner {
    pub fn run<F>(build_app: F)
    where
        F: FnOnce(&WorkloadBuilder),
    {
        let world = shipyard::World::new();
        let builder = WorkloadBuilder::new(&world);
        build_app(&builder);
        builder.build();

        let mut runner = Self {
            state: RunnerState::Waiting(world),
        };

        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut runner).unwrap();
    }
}

impl ApplicationHandler for Runner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::trace!("App Resumed - Creating inner app");

        let world = match &mut self.state {
            RunnerState::Waiting(world) => {
                let mut new_world = shipyard::World::new();
                std::mem::swap(world, &mut new_world);
                new_world
            }
            RunnerState::Running(..) => {
                log::warn!("Application resumed again...");
                return;
            }
        };

        let inner = RunnerInner::new(event_loop, world);
        self.state = RunnerState::Running(inner);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let RunnerState::Running(inner) = &mut self.state {
            inner.window_event(event_loop, window_id, event);
        };
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let RunnerState::Running(inner) = &mut self.state {
            if let StartCause::ResumeTimeReached { .. } = cause {
                inner.resumed()
            }
        }
    }

    // TODO
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let RunnerState::Running(inner) = &mut self.state {
            inner.device_event(event_loop, device_id, event);
        }
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

pub struct RunnerInner {
    world: shipyard::World,
    timestep: Duration,
}

impl RunnerInner {
    fn new(event_loop: &ActiveEventLoop, world: shipyard::World) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        world.run_with_data(window::sys_add_window, window);

        match world.run_workload(Stages::Setup) {
            Ok(_) => {}
            Err(e) => match e {
                shipyard::error::RunWorkload::Run((system, err)) => {
                    panic!(
                        "Workload setup failed to run system '{:?}'.\n\tErr: {:?}",
                        system, err
                    )
                }
                _ => panic!("Workload setup failed to run: {:?}", e),
            },
        }

        Self {
            world,
            timestep: Duration::from_secs_f32(TIMESTEP),
        }
    }

    // TODO
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

                event_loop
                    .set_control_flow(winit::event_loop::ControlFlow::wait_duration(self.timestep));
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
                    self.world.run_with_data(
                        tools::sys_process_input::<winit::keyboard::KeyCode>,
                        (key, event.state.is_pressed()),
                    );
                }
            }

            WindowEvent::MouseInput { state, button, .. } => self.world.run_with_data(
                tools::sys_process_input::<winit::event::MouseButton>,
                (button, state.is_pressed()),
            ),

            WindowEvent::CursorMoved { position, .. } => self.world.run_with_data(
                tools::sys_process_mouse_pos,
                [position.x as f32, position.y as f32],
            ),

            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(h, v) => {
                    self.world.run_with_data(tools::sys_process_wheel, [h, v])
                }
                winit::event::MouseScrollDelta::PixelDelta(_) => {}
            },

            _ => {}
        }
    }

    fn resumed(&mut self) {
        self.world
            .run(|window: shipyard::UniqueView<window::Window>| window.request_redraw());
    }

    // TODO
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }
}

impl RunnerInner {
    fn resize(&mut self, new_size: Size<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            log::warn!("Resize width or height of '0' provided");
            return;
        }

        self.world.run_with_data(window::sys_resize, new_size);
    }

    fn tick(&mut self) {
        self.world.run_workload(Stages::First).unwrap();

        cabat_shipyard::activate_events(&self.world);

        // TODO
        // self.world.run_workload(Stages::FixedUpdate).unwrap();

        self.world.run_workload(Stages::Update).unwrap();
        self.world.run_workload(Stages::Render).unwrap();
        self.world.run_workload(Stages::Last).unwrap();
    }
}

//====================================================================
