use bitvec::{bitvec, vec::BitVec};
use ndarray::{Array, Ix3};
use rand::{
    distributions::Distribution, distributions::WeightedIndex, rngs::StdRng, Rng, SeedableRng,
};
use std::{rc::Rc, sync::mpsc};

#[cfg(feature = "debug-traces")]
use tracing::info;

use crate::{
    grid::{
        direction::{Cartesian2D, DirectionSet},
        Grid, GridData,
    },
    ProcGenError,
};

use self::{
    builder::{GeneratorBuilder, Unset},
    node::{GeneratedNode, ModelIndex},
    observer::GenerationUpdate,
    rules::Rules,
};

pub mod builder;
pub mod node;
pub mod observer;
pub mod rules;

const MAX_NOISE_VALUE: f32 = 1E-6;

pub enum NodeSelectionHeuristic {
    MinimumRemainingValue,
    Random,
}

pub enum ModelSelectionHeuristic {
    WeightedProbability,
}

pub enum RngMode {
    /// The generator will use the given seed for its random source.
    Seeded(u64),
    /// The generator will use a random seed for its random source. The randomly generated seed can be retrieved by calling `get_seed` on the generator.
    RandomSeed,
}

pub enum GenerationStatus {
    Ongoing,
    Done,
}

struct PropagationEntry {
    node_index: usize,
    model_index: ModelIndex,
}

pub struct Generator<T: DirectionSet + Clone> {
    // Read-only configuration
    grid: Grid<T>,
    rules: Rc<Rules<T>>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,

    // Internal
    rng: StdRng,
    seed: u64,

    // Generation state
    /// `nodes[node_index * self.rules.models_count() + model_index]` is true (1) if model with index `model_index` is still allowed on node with index `node_index`
    nodes: BitVec<usize>,
    /// Stores how many models are still possible for a given node
    possible_models_count: Vec<usize>,
    observers: Vec<mpsc::Sender<GenerationUpdate>>,

    // Constraint satisfaction algorithm data
    /// Stack of bans to propagate
    propagation_stack: Vec<PropagationEntry>,
    /// The value at `support_count[node_index][model_index][direction]` represents the number of supports of a `model_index` at `node_index` from `direction`
    supports_count: Array<usize, Ix3>,
}

impl<T: DirectionSet + Clone> Generator<T> {
    pub fn builder() -> GeneratorBuilder<Unset, Unset, Cartesian2D> {
        GeneratorBuilder::new()
    }

    fn new(
        rules: Rc<Rules<T>>,
        grid: Grid<T>,
        max_retry_count: u32,
        node_selection_heuristic: NodeSelectionHeuristic,
        model_selection_heuristic: ModelSelectionHeuristic,
        rng_mode: RngMode,
    ) -> Self {
        let models_count = rules.models_count();
        let nodes_count = grid.total_size();
        let direction_count = grid.directions().len();

        let seed = match rng_mode {
            RngMode::Seeded(seed) => seed,
            RngMode::RandomSeed => rand::thread_rng().gen::<u64>(),
        };

        let mut generator = Self {
            grid,
            rules,
            max_retry_count,
            node_selection_heuristic,
            model_selection_heuristic,

            rng: StdRng::seed_from_u64(seed),
            seed,

            nodes: bitvec![1; nodes_count * models_count],
            possible_models_count: vec![models_count; nodes_count],
            observers: Vec::new(),

            propagation_stack: Vec::new(),
            supports_count: Array::zeros((nodes_count, models_count, direction_count)),
        };
        generator.initialize_supports_count();
        generator
    }

    pub fn get_seed(&self) -> u64 {
        self.seed
    }

    fn reinitialize(&mut self) {
        self.nodes = bitvec![1;self.rules.models_count()* self.grid.total_size() ];
        self.possible_models_count = vec![self.rules.models_count(); self.grid.total_size()];
        self.propagation_stack = Vec::new();
        self.initialize_supports_count();
    }

    fn initialize_supports_count(&mut self) {
        for node in 0..self.grid.total_size() {
            for model in 0..self.rules.models_count() {
                for direction in self.grid.directions() {
                    let opposite_dir = direction.opposite();
                    let grid_pos = self.grid.get_position(node);
                    // During initialization, the support count for a model from a direction is simply his total count of allowed adjacent models in the opposite direction (or 0 for a non-looping border).
                    self.supports_count[(node, model, *direction as usize)] =
                        match self.grid.get_next_index(&grid_pos, opposite_dir) {
                            Some(_) => self.rules.supported_models(model, opposite_dir).len(),
                            None => 0,
                        };
                }
            }
        }
    }

    pub fn generate(&mut self) -> Result<GridData<T, GeneratedNode>, ProcGenError> {
        for _i in 1..self.max_retry_count + 1 {
            // TODO Split generation in multiple blocks
            match self.generate_all_nodes() {
                Ok(_) => return Ok(self.to_grid_data()),
                Err(ProcGenError::GenerationFailure) => {
                    self.reinitialize();
                }
            }
        }
        Err(ProcGenError::GenerationFailure)
    }

    fn generate_all_nodes(&mut self) -> Result<(), ProcGenError> {
        for _i in 0..self.grid.total_size() {
            match self.select_and_propagate() {
                Ok(GenerationStatus::Done) => break,
                Ok(GenerationStatus::Ongoing) => (),
                Err(e) => return Err(e),
            };
        }
        Ok(())
    }

    pub fn select_and_propagate(&mut self) -> Result<GenerationStatus, ProcGenError> {
        let node_index = match self.select_node_to_generate() {
            Some(index) => index,
            None => return Ok(GenerationStatus::Done),
        };
        // We found a node not yet generated. "Observe/collapse" the node: select a model for the node
        let selected_model_index = self.select_model(node_index);

        // Iterate all the possible models because we don't have an easy way to iterate only the models possible at node_index. But we'll filter impossible models right away. TODO: iter_ones ?
        for model_index in 0..self.rules.models_count() {
            if model_index == selected_model_index {
                continue;
            }
            if !self.is_model_possible(node_index, model_index) {
                continue;
            }

            // Enqueue removal for propagation
            self.propagation_stack.push(PropagationEntry {
                node_index,
                model_index,
            });

            // None of these model are possible on this node now, set their support to 0
            for dir in self.grid.directions() {
                let supports_count =
                    &mut self.supports_count[(node_index, model_index, *dir as usize)];
                *supports_count = 0;
            }
        }
        // Remove eliminated possibilities (after enqueuing the propagation entries)
        // TODO Remove alias ?
        for mut bit in self.nodes[node_index * self.rules.models_count()
            ..node_index * self.rules.models_count() + self.rules.models_count()]
            .iter_mut()
        {
            *bit = false;
        }
        self.nodes.set(
            node_index * self.rules.models_count() + selected_model_index,
            true,
        );
        self.possible_models_count[node_index] = 1;

        self.signal_observers(node_index, selected_model_index);

        self.propagate()?;

        Ok(GenerationStatus::Ongoing)
    }

    fn signal_observers(&mut self, node_index: usize, model_index: ModelIndex) {
        #[cfg(feature = "debug-traces")]
        info!(
            "Select model {} for node {} at position {:?}",
            model_index,
            node_index,
            self.grid.get_position(node_index)
        );

        let update = GenerationUpdate {
            node_index,
            generated_node: self.rules.model(model_index).to_generated(),
        };
        for obs in &mut self.observers {
            let _ = obs.send(update);
        }
    }

    fn propagate(&mut self) -> Result<(), ProcGenError> {
        // Clone the Rc to allow for mutability in the interior loops
        let rules = Rc::clone(&self.rules);

        while let Some(from) = self.propagation_stack.pop() {
            let from_position = self.grid.get_position(from.node_index);
            // We want to update all the adjacent nodes (= in all directions)
            for dir in self.grid.directions() {
                // Get the adjacent node in this direction, it may not exist.
                if let Some(to_node_index) = self.grid.get_next_index(&from_position, *dir) {
                    // Decrease the support count of all models previously supported by "from"
                    for &model in rules.supported_models(from.model_index, *dir) {
                        let supports_count =
                            &mut self.supports_count[(to_node_index, model, *dir as usize)];
                        if *supports_count > 0 {
                            *supports_count -= 1;
                            // When we find a model which is now unsupported, we queue a ban
                            // We check for == because we only want to queue the event once.
                            if *supports_count == 0 {
                                self.ban_model_from_node(to_node_index, model)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn ban_model_from_node(
        &mut self,
        node_index: usize,
        model_index: usize,
    ) -> Result<(), ProcGenError> {
        #[cfg(feature = "debug-traces")]
        info!(
            "Ban model {} for node {} at position {:?}",
            model_index,
            node_index,
            self.grid.get_position(node_index)
        );

        // Enqueue removal for propagation
        self.propagation_stack.push(PropagationEntry {
            node_index,
            model_index,
        });
        // Update the supports
        for dir in self.grid.directions() {
            let supports_count = &mut self.supports_count[(node_index, model_index, *dir as usize)];
            *supports_count = 0;
        }
        // Update the state
        self.nodes
            .set(node_index * self.rules.models_count() + model_index, false);

        let number_of_models_left = &mut self.possible_models_count[node_index];
        *number_of_models_left = number_of_models_left.saturating_sub(1);
        match *number_of_models_left {
            0 => Err(ProcGenError::GenerationFailure),
            1 => {
                // This node has collapsed into a specific model
                self.signal_observers(node_index, self.get_model_index(node_index));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn select_node_to_generate<'a>(&mut self) -> Option<usize> {
        // Pick a node according to the heuristic
        match self.node_selection_heuristic {
            NodeSelectionHeuristic::MinimumRemainingValue => {
                let mut min = f32::MAX;
                let mut picked_node = None;
                for (index, &count) in self.possible_models_count.iter().enumerate() {
                    // If the node is not generated yet (multiple possibilities)
                    if count > 1 {
                        // Noise added to entropy so that when evaluating multiples candidates with the same value, we pick a random one, not in the evaluating order.
                        let noise = MAX_NOISE_VALUE * self.rng.gen::<f32>();
                        if (count as f32 + noise) < min {
                            min = count as f32 + noise;
                            picked_node = Some(index);
                        }
                    }
                }
                picked_node
            }
            NodeSelectionHeuristic::Random => {
                let mut picked_node = None;
                let mut candidates = Vec::new();
                for (index, &count) in self.possible_models_count.iter().enumerate() {
                    if count > 1 {
                        candidates.push(index);
                    }
                }
                if candidates.len() > 0 {
                    picked_node = Some(candidates[self.rng.gen_range(0..candidates.len())]);
                }
                picked_node
            }
        }
    }

    fn select_model(&mut self, node_index: usize) -> usize {
        match self.model_selection_heuristic {
            ModelSelectionHeuristic::WeightedProbability => {
                let possible_models: Vec<ModelIndex> = (0..self.rules.models_count())
                    .filter(|&model_index| self.is_model_possible(node_index, model_index))
                    .collect();

                // TODO May cache the current sum of weights at each node.
                let weighted_distribution = WeightedIndex::new(
                    possible_models
                        .iter()
                        .map(|&model_index| self.rules.weight(model_index)),
                )
                .unwrap();
                possible_models[weighted_distribution.sample(&mut self.rng)]
            }
        }
    }

    #[inline]
    fn is_model_possible(&self, node_index: usize, model_index: usize) -> bool {
        self.nodes[node_index * self.rules.models_count() + model_index] == true
    }

    /// Should only be called when the nodes are fully generated
    fn to_grid_data(&self) -> GridData<T, GeneratedNode> {
        let mut generated_nodes = Vec::with_capacity(self.nodes.len());
        for node_index in 0..self.grid.total_size() {
            let model_index = self.get_model_index(node_index);
            let expanded_model = self.rules.model(model_index);
            generated_nodes.push(expanded_model.to_generated())
        }

        GridData::new(self.grid.clone(), generated_nodes)
    }

    fn get_model_index(&self, node_index: usize) -> usize {
        self.nodes[node_index * self.rules.models_count()
            ..node_index * self.rules.models_count() + self.rules.models_count()]
            .first_one()
            .unwrap_or(0)
    }

    fn add_observer_queue(&mut self, sender: mpsc::Sender<GenerationUpdate>) {
        self.observers.push(sender);
    }
}
