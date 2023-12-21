use bevy::{
    ecs::{
        query::With,
        system::{Query, Res},
    },
    input::{keyboard::KeyCode, Input},
    render::view::Visibility,
};
use bevy_ghx_proc_gen::grid::DebugGridMesh;

pub fn toggle_debug_grid_visibility(
    keys: Res<Input<KeyCode>>,
    mut debug_grids: Query<&mut Visibility, With<DebugGridMesh>>,
) {
    if keys.just_pressed(KeyCode::F1) {
        for mut view_visibility in debug_grids.iter_mut() {
            *view_visibility = match *view_visibility {
                Visibility::Inherited => Visibility::Hidden,
                Visibility::Hidden => Visibility::Visible,
                Visibility::Visible => Visibility::Hidden,
            }
        }
    }
}
