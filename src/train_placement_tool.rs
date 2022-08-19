use bevy::utils::FloatOrd;
use bevy_prototype_lyon::prelude::tess::{geom::CubicBezierSegment, math::Point};

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

    // Projection
    v + (w - v).to_f32() * t
}

const ITERATIONS: i32 = 4;
pub fn nearest_on_curve(curve: CubicBezierSegment<f32>, point: Point) -> Point {
    let mut base = 0.5;
    for iteration in 0..ITERATIONS {
        let granularity = f32::powi(10., iteration + 1);
        let mut best_dist = f32::INFINITY;
        let mut best_base = 0.;
        for step in 0..=10 {
            let step = step as f32 - 5.;
            let t = step / granularity;
            let sample = curve.sample(base + t);
            let distance = sample.distance_to(point);
            if distance < best_dist {
                best_dist = distance;
                best_base = base + t;
            }
        }
        base = best_base;
    }
    let base = base.clamp(0.0, 1.0);

    // Attempt to get more accurate via projection, very flaky
    // let final_granularity = f32::powi(10., ITERATIONS);
    // let start = (base - final_granularity).clamp(0.0, 1.0);
    // let end = (base + final_granularity).clamp(0.0, 1.0);
    // let split_curve = curve.split_range(start..end);
    // let line = split_curve.baseline();
    // let projected = project_onto_line(line.from, line.to, point);
    // let x_points = split_curve.solve_t_for_x(projected.x);
    // let y_points = split_curve.solve_t_for_y(projected.y);
    // let points = x_points.into_iter().chain(y_points.into_iter());
    // let closest = points.min_by_key(|t| FloatOrd(curve.sample(*t).distance_to(point)));
    // if let Some(closest) = closest {
    //     println!("Got more accurate");
    //     curve.sample(closest)
    // } else {
    //     curve.sample(base)
    // }

    curve.sample(base)
}

pub fn train_placement_tool(
    mut commands: Commands,
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
            Some((id, track))
        } else {
            None
        }
    });
    let refined = filtered.map(|(_, track)| nearest_on_curve(track.curve, mouse_point));

    let min = refined.min_by_key(|pos| FloatOrd(pos.distance_to(mouse_point)));

    if let Some(point) = min {
        if point.distance_to(mouse_point) < 100. {
            draw_train(
                &mut commands,
                Vec2::new(point.x, point.y),
                Color::rgba(0.0, 0.0, 1.0, 0.95),
            );
        }
    }
}

pub fn cleanup_train_placement(mut commands: Commands, ghosts: Query<Entity, With<TrainGhost>>) {
    ghosts.for_each(|g| commands.entity(g).despawn());
}
