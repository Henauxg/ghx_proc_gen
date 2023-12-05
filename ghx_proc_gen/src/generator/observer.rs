use crate::grid::{direction::DirectionSet, GridData};

use super::{node::GeneratedNode, Generator};

#[derive(Clone, Copy, Debug)]
pub enum GenerationUpdate {
    /// A node has been generated
    Generated {
        /// Index of the node in the [`crate::grid::GridDefinition`]
        node_index: usize,
        /// Generated node info
        generated_node: GeneratedNode,
    },
    /// The generator has reinitialized from its initial state.
    Reinitialized,
    /// The generation failed due to a contradiction.
    Failed,
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
    pub fn dequeue_all(&mut self) {
        while let Ok(update) = self.receiver.try_recv() {
            match update {
                GenerationUpdate::Generated {
                    node_index,
                    generated_node,
                } => self.grid_data.set(node_index, Some(generated_node)),
                GenerationUpdate::Reinitialized => self.grid_data.reset(None),
                GenerationUpdate::Failed => self.grid_data.reset(None),
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
                    GenerationUpdate::Generated {
                        node_index,
                        generated_node,
                    } => self.grid_data.set(node_index, Some(generated_node)),
                    GenerationUpdate::Reinitialized => self.grid_data.reset(None),
                    GenerationUpdate::Failed => self.grid_data.reset(None),
                }
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
