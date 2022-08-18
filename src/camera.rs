use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

pub fn camera_zoom(
    mut cameras: Query<(&mut OrthographicProjection, &mut Transform), With<Camera>>,
    mut scroll_events: EventReader<MouseWheel>,
    windows: Res<Windows>,
) {
    let pixels_per_line = 100.; // Maybe make configurable?
    let scroll = scroll_events
        .iter()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Pixel => ev.y,
            MouseScrollUnit::Line => ev.y * pixels_per_line,
        })
        .sum::<f32>();

    if scroll == 0. {
        return;
    }

    let window = windows.get_primary().unwrap();
    let window_size = Vec2::new(window.width(), window.height());
    let mouse_normalized_screen_pos =
        (window.cursor_position().unwrap() / window_size) * 2. - Vec2::ONE;

    for (mut proj, mut pos) in &mut cameras {
        let old_scale = proj.scale;
        proj.scale = proj.scale * (1. + -scroll * 0.001); //.max(cam.min_scale);

        // if let Some(max_scale) = cam.max_scale {
        //     proj.scale = proj.scale.min(max_scale);
        // }

        let proj_size = Vec2::new(proj.right, proj.top);
        let mouse_world_pos =
            pos.translation.truncate() + mouse_normalized_screen_pos * proj_size * old_scale;
        pos.translation = (mouse_world_pos - mouse_normalized_screen_pos * proj_size * proj.scale)
            .extend(pos.translation.z);
    }
}

pub fn camera_pan(
    windows: Res<Windows>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut cameras: Query<(&mut Transform, &OrthographicProjection), With<Camera>>,
    mut last_pos: Local<Option<Vec2>>,
) {
    let window = windows.get_primary().unwrap();

    // Use position instead of MouseMotion, otherwise we don't get acceleration movement
    let current_pos = match window.cursor_position() {
        Some(current_pos) => current_pos,
        None => return,
    };
    let delta = current_pos - last_pos.unwrap_or(current_pos);

    for (mut transform, projection) in &mut cameras {
        if mouse_buttons.pressed(MouseButton::Middle) {
            let scaling = Vec2::new(
                window.width() / (projection.right - projection.left),
                window.height() / (projection.top - projection.bottom),
            ) * projection.scale;

            transform.translation -= (delta * scaling).extend(0.);
        }
    }
    *last_pos = Some(current_pos);
}
