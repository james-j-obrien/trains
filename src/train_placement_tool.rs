use bevy::utils::FloatOrd;
use bevy_prototype_lyon::prelude::tess::math::Point;

use super::*;

#[derive(Component)]
pub struct TrainGhost;

pub fn draw_train(commands: &mut Commands, pos: Vec2, color: Color) {
    let circle = shapes::Circle {
        radius: 16.,
        center: pos,
    };

    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &circle,
            DrawMode::Fill(FillMode::color(color)),
            Transform::from_xyz(0., 0., 200.),
        ))
        .insert(TrainGhost);
}

fn project_onto_line(v: Point, w: Point, p: Point) -> Point {
    let dist = v.distance_to(w);
    let dist = dist * dist;
    if dist == 0. {
        return v;
    }

    let t = ((p - v).dot(w - v) / dist).min(1.).max(0.);
    let projection = v + (w - v).to_f32() * t;
    projection
}

pub fn train_placement_tool(
    mut commands: Commands,
    // keys: Res<Input<KeyCode>>,
    network: Res<Network>,
    mouse_pos: Res<MousePos>,
    train: Query<Entity, With<TrainGhost>>,
) {
    train.for_each(|e| commands.entity(e).despawn());
    let mouse_pos = match mouse_pos.0 {
        Some(pos) => pos,
        None => return,
    };
    let mouse_point = Point::new(mouse_pos.x, mouse_pos.y);

    let filtered = network.tracks.iter().filter_map(|(id, track)| {
        let line = track.curve.baseline();
        let projected = project_onto_line(line.from, line.to, mouse_point);
        let dist = mouse_point.distance_to(projected);
        if dist < 200. {
            Some((id, track, projected))
        } else {
            None
        }
    });
    let refined = filtered.filter_map(|(_, track, projected)| {
        let x_point = track.curve.solve_t_for_x(projected.x);
        let y_point = track.curve.solve_t_for_y(projected.y);

        let t = match (x_point.first(), y_point.first()) {
            (Some(x), Some(y)) => Some((x + y) / 2.),
            (Some(x), None) => Some(*x),
            (None, Some(y)) => Some(*y),
            _ => None,
        };

        if let Some(t) = t {
            Some(track.curve.sample(t))
        } else {
            None
        }
    });

    let min = refined.min_by_key(|pos| FloatOrd(pos.distance_to(mouse_point)));

    if let Some(point) = min {
        draw_train(&mut commands, Vec2::new(point.x, point.y), Color::BLUE);
    }
}

pub fn cleanup_train_placement(mut commands: Commands, ghosts: Query<Entity, With<TrainGhost>>) {
    ghosts.for_each(|g| commands.entity(g).despawn());
}
