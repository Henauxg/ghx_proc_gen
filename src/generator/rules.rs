use std::collections::{HashMap, HashSet};

use ndarray::{Array, Ix1, Ix2};

use crate::grid::{Direction, DirectionSet};

use super::node::{expand_models, ExpandedNodeModel, ModelIndex, NodeModel};

pub struct GenerationRules {
    direction_set: DirectionSet,
    models: Vec<ExpandedNodeModel>,
    /// The vector `allowed_neighbours[model_index][direction]` holds all the allowed adjacent models (indexes) to `model_index` in `direction`.
    ///
    /// Calculated from expanded models.
    ///
    /// Note: this cannot be a 3d array since the third dimension is different for each element.
    allowed_neighbours: Array<Vec<usize>, Ix2>,
}

impl GenerationRules {
    pub fn new(models: Vec<NodeModel>, direction_set: DirectionSet) -> GenerationRules {
        let expanded_models = expand_models(models, &direction_set);

        // Temporary collection to reverse the relation: sockets_to_models.get(socket)[direction] will hold all the models that can be set in 'direction' from 'socket'
        let mut sockets_to_models = HashMap::new();
        let empty_in_all_directions: Array<HashSet<ModelIndex>, Ix1> =
            Array::from_elem(direction_set.dirs.len(), HashSet::new());
        for model in &expanded_models {
            for &direction in direction_set.dirs {
                let inverse_dir = direction.opposite() as usize;
                for socket in &model.sockets()[direction as usize] {
                    let allowed_neighbours = sockets_to_models
                        .entry(socket)
                        .or_insert(empty_in_all_directions.clone());
                    allowed_neighbours[inverse_dir].insert(model.index());
                }
            }
        }

        let mut allowed_neighbours = Array::from_elem(
            (expanded_models.len(), direction_set.dirs.len()),
            Vec::new(),
        );
        for model in &expanded_models {
            for &direction in direction_set.dirs {
                let mut unique_models = HashSet::new();
                for socket in &model.sockets()[direction as usize] {
                    for allowed_model in
                        &sockets_to_models.get(&socket).unwrap()[direction as usize]
                    {
                        match unique_models.insert(*allowed_model) {
                            true => allowed_neighbours[(model.index(), direction as usize)]
                                .push(*allowed_model),
                            false => (),
                        }
                    }
                }
            }
        }

        GenerationRules {
            direction_set,
            models: expanded_models,
            allowed_neighbours,
        }
    }

    #[inline]
    pub(crate) fn supported_models(
        &self,
        model_index: ModelIndex,
        direction: Direction,
    ) -> &Vec<ModelIndex> {
        &self.allowed_neighbours[(model_index, direction as usize)]
    }

    #[inline]
    pub(crate) fn weight(&self, model_index: ModelIndex) -> f32 {
        self.models[model_index].weight()
    }

    #[inline]
    pub(crate) fn directions(&self) -> &'static [Direction] {
        self.direction_set.dirs
    }

    pub fn models_count(&self) -> usize {
        self.models.len()
    }
}
