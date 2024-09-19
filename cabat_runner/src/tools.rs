//====================================================================

use std::{
    collections::HashSet,
    hash::Hash,
    time::{Duration, Instant},
};

use shipyard::{AllStoragesView, IntoWorkload, Unique};
use shipyard_shared::WindowSize;
use shipyard_tools::{prelude::*, UniqueTools};

//====================================================================

pub use winit::{event::MouseButton, keyboard::KeyCode};

//====================================================================

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(self, workload_builder: WorkloadBuilder) -> WorkloadBuilder {
        workload_builder
            .add_workload(Stages::Setup, (sys_setup_uniques).into_workload())
            .add_workload(Stages::First, (sys_update_time).into_workload())
            .add_workload(
                Stages::Last,
                (
                    sys_reset_input::<KeyCode>,
                    sys_reset_input::<MouseButton>,
                    sys_reset_mouse_input,
                )
                    .into_workload(),
            )
    }
}

fn sys_setup_uniques(all_storages: AllStoragesView) {
    all_storages
        .insert(Time::default())
        .insert(Input::<KeyCode>::default())
        .insert(Input::<MouseButton>::default())
        .insert(MouseInput::default());
}

//====================================================================

#[derive(Unique)]
pub struct Time {
    elapsed: Instant,

    last_frame: Instant,
    delta: Duration,
    delta_seconds: f32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            elapsed: Instant::now(),
            last_frame: Instant::now(),
            delta: Duration::ZERO,
            delta_seconds: 0.,
        }
    }
}

#[allow(dead_code)]
impl Time {
    #[inline]
    pub fn elapsed(&self) -> &Instant {
        &self.elapsed
    }

    #[inline]
    pub fn delta(&self) -> &Duration {
        &self.delta
    }

    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }
}

fn sys_update_time(mut time: ResMut<Time>) {
    time.delta = time.last_frame.elapsed();
    time.delta_seconds = time.delta.as_secs_f32();

    time.last_frame = Instant::now();
}

//====================================================================

#[derive(Unique, Debug)]
pub struct Input<T>
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    released: HashSet<T>,
}

impl<T> Default for Input<T>
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    fn default() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            released: HashSet::new(),
        }
    }
}

#[allow(dead_code)]
impl<T> Input<T>
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    fn add_pressed(&mut self, input: T) {
        self.pressed.insert(input);
        self.just_pressed.insert(input);
    }

    fn remove_pressed(&mut self, input: T) {
        self.pressed.remove(&input);
        self.released.insert(input);
    }

    fn reset(&mut self) {
        self.just_pressed.clear();
        self.released.clear();
    }

    #[inline]
    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    #[inline]
    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    #[inline]
    pub fn _released(&self, input: T) -> bool {
        self.released.contains(&input)
    }
}

pub fn sys_process_input<T>(input_data: (T, bool), mut input: ResMut<Input<T>>)
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    match input_data.1 {
        true => input.add_pressed(input_data.0),
        false => input.remove_pressed(input_data.0),
    }
}

fn sys_reset_input<T>(mut input: ResMut<Input<T>>)
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    input.reset();
}

//====================================================================

#[derive(Unique, Debug, Default)]
pub struct MouseInput {
    pos: glam::Vec2,
    screen_pos: glam::Vec2,
    pos_delta: glam::Vec2,
    scroll: glam::Vec2,
}

impl MouseInput {
    #[inline]
    pub fn scroll(&self) -> glam::Vec2 {
        self.scroll
    }

    #[inline]
    pub fn _pos(&self) -> glam::Vec2 {
        self.pos
    }

    #[inline]
    pub fn screen_pos(&self) -> glam::Vec2 {
        self.screen_pos
    }
}

pub fn sys_process_wheel(wheel: [f32; 2], mut mouse: ResMut<MouseInput>) {
    mouse.scroll += glam::Vec2::from(wheel);
}

pub fn sys_process_mouse_pos(pos: [f32; 2], mut mouse: ResMut<MouseInput>, size: Res<WindowSize>) {
    mouse.pos = pos.into();
    mouse.screen_pos = glam::vec2(mouse.pos.x, size.height_f32() - mouse.pos.y as f32);
}

fn sys_reset_mouse_input(mut mouse: ResMut<MouseInput>) {
    mouse.pos_delta = glam::Vec2::ZERO;
    mouse.scroll = glam::Vec2::ZERO;
}

//====================================================================
