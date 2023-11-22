use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use ndarray::{Array, Ix1, Ix2};

use crate::grid::direction::{Cartesian2D, Cartesian3D, Direction, DirectionSet};

use super::node::{expand_models, ExpandedNodeModel, ModelIndex, NodeModel};

pub struct Rules<T: DirectionSet> {
    models: Vec<ExpandedNodeModel>,
    /// The vector `allowed_neighbours[model_index][direction]` holds all the allowed adjacent models (indexes) to `model_index` in `direction`.
    ///
    /// Calculated from expanded models.
    ///
    /// Note: this cannot be a 3d array since the third dimension is different for each element.
    allowed_neighbours: Array<Vec<usize>, Ix2>,

    typestate: PhantomData<T>,
}

impl Rules<Cartesian2D> {
    pub fn new_cartesian_2d(models: Vec<NodeModel>) -> Rules<Cartesian2D> {
        Self::new(models, Cartesian2D {})
    }
}

impl Rules<Cartesian3D> {
    pub fn new_cartesian_3d(models: Vec<NodeModel>) -> Rules<Cartesian3D> {
        Self::new(models, Cartesian3D {})
    }
}

impl<T: DirectionSet> Rules<T> {
    fn new(models: Vec<NodeModel>, direction_set: T) -> Rules<T> {
        let expanded_models = expand_models(models, &direction_set);

        // Temporary collection to reverse the relation: sockets_to_models.get(socket)[direction] will hold all the models that can be set in 'direction' from 'socket'
        let mut sockets_to_models = HashMap::new();
        let empty_in_all_directions: Array<HashSet<ModelIndex>, Ix1> =
            Array::from_elem(direction_set.directions().len(), HashSet::new());
        for (model_index, model) in expanded_models.iter().enumerate() {
            for &direction in direction_set.directions() {
                let inverse_dir = direction.opposite() as usize;
                for socket in &model.sockets()[direction as usize] {
                    let compatible_models = sockets_to_models
                        .entry(socket)
                        .or_insert(empty_in_all_directions.clone());
                    compatible_models[inverse_dir].insert(model_index);
                }
            }
        }

        let mut allowed_neighbours = Array::from_elem(
            (expanded_models.len(), direction_set.directions().len()),
            Vec::new(),
        );
        for (model_index, model) in expanded_models.iter().enumerate() {
            for &direction in direction_set.directions() {
                let mut unique_models = HashSet::new();
                for socket in &model.sockets()[direction as usize] {
                    for allowed_model in
                        &sockets_to_models.get(&socket).unwrap()[direction as usize]
                    {
                        match unique_models.insert(*allowed_model) {
                            true => allowed_neighbours[(model_index, direction as usize)]
                                .push(*allowed_model),
                            false => (),
                        }
                    }
                }
            }
        }

        Rules {
            models: expanded_models,
            allowed_neighbours,
            typestate: PhantomData,
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
    pub fn models_count(&self) -> usize {
        self.models.len()
    }
}
