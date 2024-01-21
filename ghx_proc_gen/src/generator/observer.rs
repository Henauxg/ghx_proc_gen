use crate::grid::{direction::CoordinateSystem, GridData};

use super::{model::ModelInstance, Generator, GridNode};

/// Update sent by a [`crate::generator::Generator`]
#[derive(Clone, Copy, Debug)]
pub enum GenerationUpdate {
    /// A node has been generated
    Generated(GridNode),
    /// The generator is being reinitialized to its initial state, with a new seed.
    Reinitializing(u64),
    /// The generation failed due to a contradiction at the specified node_index
    Failed(usize),
}

/// Observer with a queue of the [`GenerationUpdate`] sent by the [`crate::generator::Generator`] which also maintains a coherent state of the current generation in a [`GridData`]
///
/// Can be used in a different thread than the generator's thread.
pub struct QueuedStatefulObserver<T: CoordinateSystem + Clone> {
    grid_data: GridData<T, Option<ModelInstance>>,
    receiver: crossbeam_channel::Receiver<GenerationUpdate>,
}

impl<T: CoordinateSystem + Clone> QueuedStatefulObserver<T> {
    /// Creates a new [`QueuedStatefulObserver`] for a given [`crate::generator::Generator`]
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

    /// Returns a ref to the observer's [`GridData`]
    pub fn grid_data(&self) -> &GridData<T, Option<ModelInstance>> {
        &self.grid_data
    }

    /// Updates the internal state of the observer by dequeuing all queued updates.
    pub fn dequeue_all(&mut self) {
        while let Ok(update) = self.receiver.try_recv() {
            match update {
                GenerationUpdate::Generated(grid_node) => self
                    .grid_data
                    .set(grid_node.node_index, Some(grid_node.model_instance)),
                GenerationUpdate::Reinitializing(_) => self.grid_data.reset(None),
                GenerationUpdate::Failed(_) => self.grid_data.reset(None),
            }
        }
    }

    /// Updates the internal state of the observer by dequeuing 1 queued update.
    ///
    /// Returns [`Some(GenerationUpdate)`] if there was an update to process, else returns `None`.
    pub fn dequeue_one(&mut self) -> Option<GenerationUpdate> {
        match self.receiver.try_recv() {
            Ok(update) => {
                match update {
                    GenerationUpdate::Generated(grid_node) => self
                        .grid_data
                        .set(grid_node.node_index, Some(grid_node.model_instance)),
                    GenerationUpdate::Reinitializing(_) => self.grid_data.reset(None),
                    GenerationUpdate::Failed(_) => self.grid_data.reset(None),
                }
                Some(update)
            }
            Err(_) => None,
        }
    }
}

/// Observer with just a queue of the [`GenerationUpdate`] sent by the [`crate::generator::Generator`]
///
/// Can be used in a different thread than the generator's thread.
pub struct QueuedObserver {
    receiver: crossbeam_channel::Receiver<GenerationUpdate>,
}

impl QueuedObserver {
    /// Creates a new [`QueuedObserver`] for a given [`crate::generator::Generator`]
    pub fn new<T: CoordinateSystem + Clone>(generator: &mut Generator<T>) -> Self {
        let receiver = generator.add_observer_queue();
        QueuedObserver { receiver }
    }

    /// Dequeues all queued updates.
    ///
    /// Returns all retrieved [`GenerationUpdate`] in a `Vec`.
    /// The `Vec` may be empty if no update was queued.
    pub fn dequeue_all(&mut self) -> Vec<GenerationUpdate> {
        let mut updates = Vec::new();
        while let Ok(update) = self.receiver.try_recv() {
            updates.push(update);
        }
        updates
    }

    /// Dequeues 1 queued update.
    ///
    /// Returns [`Some(GenerationUpdate)`] if there was an update to process, else returns `None`.
    pub fn dequeue_one(&mut self) -> Option<GenerationUpdate> {
        match self.receiver.try_recv() {
            Ok(update) => Some(update),
            Err(_) => None,
        }
    }
}
