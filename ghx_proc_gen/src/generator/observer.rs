use crate::grid::{direction::DirectionSet, GridData};

use super::{node::GeneratedNode, Generator};

#[derive(Clone, Copy)]
pub struct GenerationUpdate {
    pub(crate) node_index: usize,
    pub(crate) generated_node: GeneratedNode,
}

impl GenerationUpdate {
    pub fn node_index(&self) -> usize {
        self.node_index
    }

    pub fn node(&self) -> GeneratedNode {
        self.generated_node
    }
}

pub struct QueuedStatefulObserver<T: DirectionSet + Clone> {
    grid_data: GridData<T, Option<GeneratedNode>>,
    receiver: crossbeam_channel::Receiver<GenerationUpdate>,
}

impl<T: DirectionSet + Clone> QueuedStatefulObserver<T> {
    pub fn new(generator: &mut Generator<T>) -> Self {
        let receiver = generator.add_observer_queue();
        QueuedStatefulObserver {
            grid_data: GridData::new(
                generator.grid.clone(),
                vec![None; generator.grid.total_size()],
            ),
            receiver,
        }
    }

    pub fn grid_data(&self) -> &GridData<T, Option<GeneratedNode>> {
        &self.grid_data
    }

    /// Updates the internal state of the observer by dequeuing all queued updates.
    pub fn update(&mut self) {
        while let Ok(update) = self.receiver.try_recv() {
            self.grid_data
                .set(update.node_index, Some(update.generated_node))
        }
    }

    /// Updates the internal state of the observer by dequeuing 1 queued update.
    ///
    /// Returns [`Some(GenerationUpdate)`] if there was an update to process, else returns `None`.
    pub fn update_one_step(&mut self) -> Option<GenerationUpdate> {
        match self.receiver.try_recv() {
            Ok(update) => {
                self.grid_data
                    .set(update.node_index, Some(update.generated_node));
                Some(update)
            }
            Err(_) => None,
        }
    }
}

pub struct QueuedObserver {
    receiver: crossbeam_channel::Receiver<GenerationUpdate>,
}

impl QueuedObserver {
    pub fn new<T: DirectionSet + Clone>(generator: &mut Generator<T>) -> Self {
        let receiver = generator.add_observer_queue();
        QueuedObserver { receiver }
    }

    /// Dequeues all queued updates.
    ///
    /// Returns all retrieved [`GenerationUpdate`] in a `Vec`.
    /// The `Vec` may be empty if no update was queued.
    pub fn update(&mut self) -> Vec<GenerationUpdate> {
        let mut updates = Vec::new();
        while let Ok(update) = self.receiver.try_recv() {
            updates.push(update);
        }
        updates
    }

    /// Dequeues 1 queued update.
    ///
    /// Returns [`Some(GenerationUpdate)`] if there was an update to process, else returns `None`.
    pub fn update_one_step(&mut self) -> Option<GenerationUpdate> {
        match self.receiver.try_recv() {
            Ok(update) => Some(update),
            Err(_) => None,
        }
    }
}
