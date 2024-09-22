//====================================================================

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use shipyard::{info::TypeId, Unique, WorkloadModificator, World};

//====================================================================

pub mod prelude {
    pub use crate::{Event, EventHandler, Plugin, Res, ResMut, Stages, SubStages, WorkloadBuilder};
}

//====================================================================

pub type Res<'a, T> = shipyard::UniqueView<'a, T>;
pub type ResMut<'a, T> = shipyard::UniqueViewMut<'a, T>;

//====================================================================

#[allow(dead_code)]
pub trait WorldTools {
    fn and_run<B, S: shipyard::System<(), B>>(&self, system: S) -> &Self;
    fn and_run_with_data<Data, B, S: shipyard::System<(Data,), B>>(
        &self,
        system: S,
        data: Data,
    ) -> &Self;
}

impl WorldTools for shipyard::World {
    #[inline]
    fn and_run<B, S: shipyard::System<(), B>>(&self, system: S) -> &Self {
        self.run(system);
        self
    }

    #[inline]
    fn and_run_with_data<Data, B, S: shipyard::System<(Data,), B>>(
        &self,
        system: S,
        data: Data,
    ) -> &Self {
        self.run_with_data(system, data);
        self
    }
}

#[allow(dead_code)]
pub trait UniqueTools {
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self;
    fn replace<U: shipyard::Unique + Send + Sync>(&self, unique: U);
}

impl UniqueTools for shipyard::World {
    #[inline]
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self {
        self.add_unique(unique);
        self
    }

    fn replace<U: shipyard::Unique + Send + Sync>(&self, unique: U) {
        self.remove_unique::<U>().ok();
        self.add_unique(unique);
    }
}

impl UniqueTools for shipyard::AllStoragesView<'_> {
    #[inline]
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self {
        self.add_unique(unique);
        self
    }

    fn replace<U: shipyard::Unique + Send + Sync>(&self, unique: U) {
        self.remove_unique::<U>().ok();
        self.add_unique(unique);
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

pub struct WorkloadBuilder<'a> {
    world: &'a shipyard::World,
    workloads: HashMap<Stages, WorkloadToBuild>,
    event_workloads: HashMap<TypeId, shipyard::Workload>,

    plugin_text: String,
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

impl<'a> WorkloadBuilder<'a> {
    pub fn new(world: &'a shipyard::World) -> Self {
        Self {
            world,
            workloads: HashMap::new(),
            event_workloads: HashMap::new(),

            plugin_text: String::new(),
        }
    }

    pub fn add_workload_sub(
        mut self,
        stage: Stages,
        substage: SubStages,
        workload: shipyard::Workload,
    ) -> Self {
        self.plugin_text = format!(
            "{}\n\tAdding workload for stage '{:?}' - substage {:?}",
            self.plugin_text, stage, substage
        );

        let mut old_workload = self
            .workloads
            .remove(&stage)
            .unwrap_or(WorkloadToBuild::new(stage));

        let new_substage = match old_workload.substages.remove(&substage) {
            Some(old_substage) => old_substage.merge(workload),
            None => workload,
        };

        old_workload.substages.insert(substage, new_substage);

        self.workloads.insert(stage, old_workload);
        self
    }

    #[inline]
    pub fn add_workload_first(self, stage: Stages, workload: shipyard::Workload) -> Self {
        self.add_workload_sub(stage, SubStages::First, workload)
    }

    #[inline]
    pub fn add_workload_pre(self, stage: Stages, workload: shipyard::Workload) -> Self {
        self.add_workload_sub(stage, SubStages::Pre, workload)
    }

    #[inline]
    pub fn add_workload(self, stage: Stages, workload: shipyard::Workload) -> Self {
        self.add_workload_sub(stage, SubStages::Main, workload)
    }

    #[inline]
    pub fn add_workload_post(self, stage: Stages, workload: shipyard::Workload) -> Self {
        self.add_workload_sub(stage, SubStages::Post, workload)
    }

    #[inline]
    pub fn add_workload_last(self, stage: Stages, workload: shipyard::Workload) -> Self {
        self.add_workload_sub(stage, SubStages::Last, workload)
    }

    pub fn add_event<E: Event>(mut self, workload: shipyard::Workload) -> Self {
        let id = TypeId::of::<E>();

        let old_workload = self
            .event_workloads
            .remove(&id)
            .unwrap_or(shipyard::Workload::new(id));

        self.event_workloads
            .insert(id, old_workload.merge(workload));

        self
    }

    pub fn add_plugin<T: Plugin>(mut self, plugin: T) -> Self {
        self.plugin_text = format!("Adding plugin '{}'", std::any::type_name::<T>());
        let builder = plugin.build(self);

        log::trace!("{}", builder.plugin_text);

        builder
    }

    pub fn build(self) {
        self.workloads.into_iter().for_each(|(_, mut to_build)| {
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

                            //
                        }
                        None => acc,
                    }
                })
                .add_to_world(&self.world)
                .unwrap();
        });

        // Print debug data
        let data = self.world.workloads_info().0.iter().fold(
            String::from("Building workloads. Registered Stages and functions:"),
            |acc, (name, workload_info)| {
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
        let ids = self
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
    }
}

pub trait Plugin {
    fn build(self, workload_builder: WorkloadBuilder) -> WorkloadBuilder;
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

pub fn activate_events(world: &World) {
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
