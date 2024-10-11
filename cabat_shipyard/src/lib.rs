//====================================================================

use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use shipyard::{info::TypeId, IntoWorkload, Unique, UniqueView, WorkloadModificator};

//====================================================================

pub mod prelude {
    pub use crate::{Event, EventHandler, Plugin, Res, ResMut, Stages, SubStages, WorkloadBuilder};
}

//====================================================================

pub type Res<'a, T> = shipyard::UniqueView<'a, T>;
pub type ResMut<'a, T> = shipyard::UniqueViewMut<'a, T>;

//====================================================================

pub trait GetWorld {
    fn get_world(&self) -> &shipyard::World;
}

#[allow(dead_code)]
pub trait WorldTools {
    fn and_run<B, S: shipyard::System<(), B>>(&self, system: S) -> &Self;

    fn and_run_with_data<Data, B, S: shipyard::System<(Data,), B>>(
        &self,
        system: S,
        data: Data,
    ) -> &Self;
}

#[allow(dead_code)]
pub trait UniqueTools {
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self;
    fn insert_default<U: shipyard::Unique + Send + Sync + Default>(&self) -> &Self {
        self.insert(U::default())
    }

    fn get_or_insert<U, F>(&self, insert: F) -> UniqueView<'_, U>
    where
        U: shipyard::Unique + Send + Sync,
        F: Fn() -> U;
}

//--------------------------------------------------

impl GetWorld for shipyard::World {
    #[inline]
    fn get_world(&self) -> &shipyard::World {
        &self
    }
}

impl<T: GetWorld> WorldTools for T {
    #[inline]
    fn and_run<B, S: shipyard::System<(), B>>(&self, system: S) -> &Self {
        self.get_world().run(system);
        self
    }

    #[inline]
    fn and_run_with_data<Data, B, S: shipyard::System<(Data,), B>>(
        &self,
        system: S,
        data: Data,
    ) -> &Self {
        self.get_world().run_with_data(system, data);
        self
    }
}

impl<T: GetWorld> UniqueTools for T {
    #[inline]
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self {
        self.get_world().add_unique(unique);
        self
    }

    fn get_or_insert<U, F>(&self, insert: F) -> UniqueView<'_, U>
    where
        U: shipyard::Unique + Send + Sync,
        F: Fn() -> U,
    {
        match self.get_world().get_unique::<&U>() {
            Ok(unique) => return unique,

            Err(shipyard::error::GetStorage::MissingStorage { .. }) => {
                // Add and return new unique
                self.get_world().add_unique(insert());
                return self.get_world().get_unique::<&U>().unwrap();
            }

            Err(_) => std::unimplemented!(),
        };
    }
}

//====================================================================

impl UniqueTools for shipyard::AllStoragesView<'_> {
    #[inline]
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self {
        self.add_unique(unique);
        self
    }

    fn get_or_insert<U, F>(&self, insert: F) -> UniqueView<'_, U>
    where
        U: shipyard::Unique + Send + Sync,
        F: Fn() -> U,
    {
        match self.get_unique::<&U>() {
            Ok(unique) => return unique,

            Err(shipyard::error::GetStorage::MissingStorage { .. }) => {
                // Add and return new unique
                self.add_unique(insert());
                return self.get_unique::<&U>().unwrap();
            }

            Err(_) => std::unimplemented!(),
        };
    }
}

//====================================================================

#[derive(shipyard::Label, Hash, Debug, Clone, Copy, PartialEq, Eq, enum_iterator::Sequence)]
pub enum Stages {
    Setup,
    First,
    FixedUpdate,
    Update,
    Render,
    Last,
}

#[derive(shipyard::Label, Hash, Debug, Clone, Copy, PartialEq, Eq, enum_iterator::Sequence)]
pub enum SubStages {
    First,
    Pre,
    Main,
    Post,
    Last,
}

impl Iterator for SubStages {
    type Item = SubStages;

    fn next(&mut self) -> Option<Self::Item> {
        let next = enum_iterator::Sequence::next(self);

        if let Some(next) = next {
            *self = next;
        }
        next
    }
}

//====================================================================

pub trait Plugin {
    fn build(self, builder: &WorkloadBuilder);
}

//--------------------------------------------------

pub struct WorkloadBuilder<'a> {
    world: &'a shipyard::World,
    inner: RefCell<WorkloadBuilderInner>,
}

struct WorkloadBuilderInner {
    workloads: HashMap<Stages, WorkloadToBuild>,
    event_workloads: HashMap<TypeId, shipyard::Workload>,
    event_workload_names: HashMap<String, String>, // Type ID : Event name

    build_tabs: u8,
    build_text: String,
}

struct WorkloadToBuild {
    main: shipyard::Workload,
    substages: HashMap<SubStages, shipyard::Workload>,
}

impl WorkloadToBuild {
    fn new(stage: Stages) -> Self {
        Self {
            main: shipyard::Workload::new(stage),
            substages: HashMap::new(),
        }
    }
}

//--------------------------------------------------

// TODO - Add support for Asset Storage
impl<'a> WorkloadBuilder<'a> {
    pub fn new(world: &'a shipyard::World) -> Self {
        let inner = WorkloadBuilderInner {
            workloads: HashMap::new(),
            event_workloads: HashMap::new(),
            event_workload_names: HashMap::new(),

            build_tabs: 0,
            build_text: "Setting up Workload Builder".to_string(),
        };

        Self {
            world,
            inner: RefCell::new(inner),
        }
    }

    pub fn build(self) {
        let inner = self.inner.into_inner();

        log::trace!("{}", inner.build_text);

        inner.workloads.into_iter().for_each(|(_, mut to_build)| {
            enum_iterator::all::<SubStages>()
                .into_iter()
                .fold(to_build.main, |acc, substage| {
                    // Check and Add substage if it exists
                    match to_build.substages.remove(&substage) {
                        Some(workload) => {
                            let workload = workload.tag(substage);
                            // Go through substages and add before all
                            acc.merge(substage.into_iter().fold(workload, |acc, substage_after| {
                                acc.before_all(substage_after)
                            }))
                        }
                        None => acc,
                    }
                })
                .add_to_world(&self.world)
                .unwrap();
        });

        // Make sure all workloads exist in world, even if empty
        enum_iterator::all::<Stages>()
            .into_iter()
            .for_each(
                |stage| match shipyard::Workload::new(stage).add_to_world(&self.world) {
                    Ok(_) | Err(shipyard::error::AddWorkload::AlreadyExists) => {}
                    Err(e) => panic!("{e}"),
                },
            );

        // Process events
        let ids = inner
            .event_workloads
            .into_iter()
            .map(|(id, workload)| {
                workload.add_to_world(&self.world).unwrap();
                id
            })
            .collect::<Vec<_>>();

        let event_handler = EventHandler {
            event_subscribers: ids,
            ..Default::default()
        };

        self.world.add_unique(event_handler);

        // Print debug data
        let data = self.world.workloads_info().0.iter().fold(
            String::from("Building workloads. Registered Stages and functions:"),
            |acc, (name, workload_info)| {
                let name = match inner.event_workload_names.get(name) {
                    Some(event_name) => event_name,
                    None => name,
                };

                let acc = format!("{}\n{}", acc, name);

                workload_info
                    .batch_info
                    .iter()
                    .fold(acc, |acc, batch_info| {
                        batch_info
                            .systems()
                            .fold(acc, |acc, system| format!("{}\n    {}", acc, system.name))
                    })
            },
        );

        log::debug!("{data}");
    }
}

impl<'a> WorkloadBuilder<'a> {
    pub fn log(&self, text: String) {
        let mut inner = self.inner.borrow_mut();

        let tabs = (0..inner.build_tabs).map(|_| "\t").collect::<String>();
        inner.build_text = format!("{}\n{}⌙ {}", inner.build_text, tabs, text);
    }
}

impl<'a> WorkloadBuilder<'a> {
    fn add_workload_sub(&self, stage: Stages, substage: SubStages, workload: shipyard::Workload) {
        self.log(format!(
            "Adding workload for stage '{:?}' - substage {:?}",
            stage, substage
        ));

        let mut inner = self.inner.borrow_mut();

        let mut old_workload = inner
            .workloads
            .remove(&stage)
            .unwrap_or(WorkloadToBuild::new(stage));

        let new_substage = match old_workload.substages.remove(&substage) {
            Some(old_substage) => old_substage.merge(workload),
            None => workload,
        };

        old_workload.substages.insert(substage, new_substage);

        inner.workloads.insert(stage, old_workload);
    }

    //--------------------------------------------------

    pub fn add_workload_first<Views, R, Sys>(&self, stage: Stages, workload: Sys) -> &Self
    where
        Sys: IntoWorkload<Views, R>,
        R: 'static,
    {
        self.add_workload_sub(stage, SubStages::First, workload.into_workload());
        self
    }

    pub fn add_workload_pre<Views, R, Sys>(&self, stage: Stages, workload: Sys) -> &Self
    where
        Sys: IntoWorkload<Views, R>,
        R: 'static,
    {
        self.add_workload_sub(stage, SubStages::Pre, workload.into_workload());
        self
    }

    pub fn add_workload<Views, R, Sys>(&self, stage: Stages, workload: Sys) -> &Self
    where
        Sys: IntoWorkload<Views, R>,
        R: 'static,
    {
        self.add_workload_sub(stage, SubStages::Main, workload.into_workload());
        self
    }

    pub fn add_workload_post<Views, R, Sys>(&self, stage: Stages, workload: Sys) -> &Self
    where
        Sys: IntoWorkload<Views, R>,
        R: 'static,
    {
        self.add_workload_sub(stage, SubStages::Post, workload.into_workload());
        self
    }

    pub fn add_workload_last<Views, R, Sys>(&self, stage: Stages, workload: Sys) -> &Self
    where
        Sys: IntoWorkload<Views, R>,
        R: 'static,
    {
        self.add_workload_sub(stage, SubStages::Last, workload.into_workload());
        self
    }

    //--------------------------------------------------

    // TODO - Find way to convert to use IntoWorkload
    pub fn add_event<E: Event>(&self, workload: shipyard::Workload) -> &Self {
        let id = TypeId::of::<E>();

        self.log(format!(
            "Adding workload for event '{}'",
            std::any::type_name::<E>()
        ));

        {
            let mut inner = self.inner.borrow_mut();

            // Get existing workload or create new one
            let old_workload = inner
                .event_workloads
                .remove(&id)
                .unwrap_or(shipyard::Workload::new(id));

            inner
                .event_workloads
                .insert(id, old_workload.merge(workload));

            // Store event type name
            inner
                .event_workload_names
                .entry(format!("{:?}", id))
                .or_insert(std::any::type_name::<E>().to_string());
        }

        self
    }

    // TODO - Add tracking to make sure plugin can't be added multiple times
    pub fn add_plugin<T: Plugin>(&self, plugin: T) -> &Self {
        self.log(format!("Adding plugin '{}'", std::any::type_name::<T>()));
        self.inner.borrow_mut().build_tabs += 1;

        plugin.build(self);
        self.inner.borrow_mut().build_tabs -= 1;
        self
    }
}

//--------------------------------------------------

impl GetWorld for WorkloadBuilder<'_> {
    #[inline]
    fn get_world(&self) -> &shipyard::World {
        &self.world
    }
}

//====================================================================

pub use cabat_proc::Event;
pub trait Event: Send + Sync + downcast::AnySync {}

#[derive(Unique, Default)]
pub struct EventHandler {
    pending: HashMap<TypeId, Box<dyn Event>>,
    active: HashMap<TypeId, Box<dyn Event>>,

    event_subscribers: Vec<TypeId>,
}

impl EventHandler {
    pub fn add_event<E: 'static + Event>(&mut self, event: E) {
        let id = TypeId::of::<E>();

        self.pending.insert(id, Box::new(event));
    }

    pub fn get_event<E: 'static + Event>(&self) -> Option<&E> {
        let id = TypeId::of::<E>();
        match self.active.get(&id) {
            Some(data) => data.deref().as_any().downcast_ref(),
            None => return None,
        }
    }
}

pub fn activate_events(world: &shipyard::World) {
    let mut handler = world.borrow::<ResMut<EventHandler>>().unwrap();

    match handler.pending.is_empty() {
        true => {
            handler.active.clear();
            return;
        }
        false => {
            let handler = handler.deref_mut();
            std::mem::swap(&mut handler.active, &mut handler.pending);
            handler.pending.clear();
        }
    }

    let keys = handler
        .active
        .keys()
        .filter_map(|key| match handler.event_subscribers.contains(key) {
            true => Some(*key),
            false => None,
        })
        .collect::<Vec<_>>();

    std::mem::drop(handler);

    keys.iter()
        .for_each(|key| world.run_workload(*key).unwrap());

    // // TODO - log event names instead of IDs
    // if !keys.is_empty() {
    //     log::trace!("Triggering events for {:?}", keys);
    // }
}

//====================================================================
