use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin},
    asset::Asset,
    ecs::bundle::Bundle,
};

use crate::grid::SharableCoordSystem;

pub struct ProcGenSimplePlugin<C: SharableCoordSystem, A: Asset, B: Bundle> {
    typestate: PhantomData<(C, A, B)>,
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> Plugin for ProcGenSimplePlugin<C, A, B> {
    fn build(&self, app: &mut App) {}
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> ProcGenSimplePlugin<C, A, B> {
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}
