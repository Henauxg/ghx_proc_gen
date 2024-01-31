use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera::Projection;

// Taken from "Pan-orbit-camera" from the Bevy Cheat Book (https://bevy-cheatbook.github.io/cookbook/pan-orbit-camera.html), updated to bevy 0.12

/// Tags an entity as capable of panning and orbiting.
#[derive(Component)]
pub struct PanOrbitCamera {
    /// The "focus point" to orbit around. It is automatically updated when panning the camera
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
    pub auto_orbit: bool,
    pub auto_orbit_factor: f32,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 5.0,
            upside_down: false,
            auto_orbit: true,
            auto_orbit_factor: 0.3,
        }
    }
}

/// Toggles the PanOrbitCamera auto orbit
///
/// ### Example
///
/// Toggles On/Off by pressing F1
/// ```rust,ignore
///  app.add_systems(
///    Update,
///    toggle_auto_orbit.run_if(input_just_pressed(KeyCode::F1)),
///  );
/// ```
pub fn toggle_auto_orbit(mut pan_orbit_camera: Query<&mut PanOrbitCamera>) {
    let mut cam = pan_orbit_camera.single_mut();
    cam.auto_orbit = !cam.auto_orbit;
}

/// Pan the camera with middle mouse click, zoom with scroll wheel, orbit with right mouse click.
pub fn pan_orbit_camera(
    window: Query<&Window>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform, &Projection)>,
) {
    let window = window.single();

    // change input mapping for orbit and panning here
    let orbit_button = MouseButton::Right;
    let pan_button = MouseButton::Middle;

    let mut pan = Vec2::ZERO;
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let mut orbit_button_changed = false;

    let mut orbit_or_pan = false;
    if input_mouse.pressed(orbit_button) {
        for ev in ev_motion.read() {
            rotation_move += ev.delta;
        }
        orbit_or_pan = true;
    } else if input_mouse.pressed(pan_button) {
        // Pan only if we're not rotating at the moment
        for ev in ev_motion.read() {
            pan += ev.delta;
        }
        orbit_or_pan = true;
    }

    for ev in ev_scroll.read() {
        scroll += ev.y;
    }

    if input_mouse.just_released(orbit_button) || input_mouse.just_pressed(orbit_button) {
        orbit_button_changed = true;
    }

    for (mut pan_orbit, mut transform, projection) in query.iter_mut() {
        if orbit_button_changed {
            // only check for upside down when orbiting started or ended this frame
            // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
            let up = transform.rotation * Vec3::Y;
            pan_orbit.upside_down = up.y <= 0.0;
        }

        // Only auto-orbit when there is no user input
        if pan_orbit.auto_orbit && scroll == 0. && !orbit_or_pan {
            rotation_move.x += pan_orbit.auto_orbit_factor;
        }

        let mut any = false;
        if rotation_move.length_squared() > 0.0 {
            any = true;
            let window_size = get_primary_window_size(&window);
            let delta_x = {
                let delta = rotation_move.x / window_size.x * std::f32::consts::PI * 2.0;
                if pan_orbit.upside_down {
                    -delta
                } else {
                    delta
                }
            };
            let delta_y = rotation_move.y / window_size.y * std::f32::consts::PI;
            let yaw = Quat::from_rotation_y(-delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);
            transform.rotation = yaw * transform.rotation; // rotate around global y axis
            transform.rotation = transform.rotation * pitch; // rotate around local x axis
        } else if pan.length_squared() > 0.0 {
            any = true;
            // make panning distance independent of resolution and FOV,
            let window_size = get_primary_window_size(&window);
            if let Projection::Perspective(projection) = projection {
                pan *= Vec2::new(projection.fov * projection.aspect_ratio, projection.fov)
                    / window_size;
            }
            // translate by local axes
            let right = transform.rotation * Vec3::X * -pan.x;
            let up = transform.rotation * Vec3::Y * pan.y;
            // make panning proportional to distance away from focus point
            let translation = (right + up) * pan_orbit.radius;
            pan_orbit.focus += translation;
        } else if scroll.abs() > 0.0 {
            any = true;
            pan_orbit.radius -= scroll * pan_orbit.radius * 0.2;
            // dont allow zoom to reach zero or you get stuck
            pan_orbit.radius = f32::max(pan_orbit.radius, 0.05);
        }

        if any {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation =
                pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
        }
    }

    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    ev_motion.clear();
}

fn get_primary_window_size(window: &Window) -> Vec2 {
    Vec2::new(window.width() as f32, window.height() as f32)
}
