use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        system::{Commands, Query, Res, Resource},
    },
    math::Vec3,
    time::Time,
    transform::components::Transform,
};

/// Used for the examples
#[derive(Component, Clone, Resource)]
pub struct SpawningScaleAnimation {
    pub duration_sec: f32,
    pub progress: f32,
    pub easing: fn(f32) -> f32,
    pub final_scale: Vec3,
}

impl SpawningScaleAnimation {
    pub fn new(duration_sec: f32, final_scale: Vec3, easing: fn(f32) -> f32) -> Self {
        Self {
            duration_sec,
            final_scale,
            easing,
            progress: 0.,
        }
    }

    pub fn advance(&mut self, delta_sec: f32) {
        self.progress += delta_sec;
    }

    pub fn ended(&self) -> bool {
        self.progress >= self.duration_sec
    }

    pub fn progress_factor(&self) -> f32 {
        self.progress / self.duration_sec
    }

    pub fn current_value(&self) -> Vec3 {
        self.final_scale * (self.easing)(self.progress_factor())
    }

    pub fn final_value(&self) -> Vec3 {
        self.final_scale
    }
}

pub fn animate_scale(
    mut commands: Commands,
    time: Res<Time>,
    mut spawning_nodes: Query<(Entity, &mut Transform, &mut SpawningScaleAnimation)>,
) {
    for (entity, mut transform, mut animation) in spawning_nodes.iter_mut() {
        animation.advance(time.delta_secs());
        if animation.ended() {
            commands.entity(entity).remove::<SpawningScaleAnimation>();
            transform.scale = animation.final_value();
        } else {
            transform.scale = animation.current_value();
        }
    }
}

pub fn ease_in_cubic(x: f32) -> f32 {
    return x * x * x;
}

pub fn ease_in_out_cubic(x: f32) -> f32 {
    if x < 0.5 {
        4. * x * x * x
    } else {
        1. - (-2. * x + 2.).powi(3) / 2.
    }
}
