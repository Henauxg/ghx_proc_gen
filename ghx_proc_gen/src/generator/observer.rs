use crate::grid::{direction::DirectionSet, GridData};

use super::{node::GeneratedNode, Generator};

#[derive(Clone, Copy)]
pub struct GenerationUpdate {
    pub(crate) node_index: usize,
    pub(crate) generated_node: GeneratedNode,
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

    pub fn update(&mut self) {
        while let Ok(update) = self.receiver.try_recv() {
            self.grid_data
                .set(update.node_index, Some(update.generated_node))
        }
    }

    /// Returns true if there was as update to process.
    pub fn update_one_step(&mut self) -> bool {
        match self.receiver.try_recv() {
            Ok(update) => {
                self.grid_data
                    .set(update.node_index, Some(update.generated_node));
                true
            }
            Err(_) => false,
        }
    }
}
