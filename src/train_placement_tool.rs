use bevy::utils::FloatOrd;
use bevy_mod_picking::{Hover, PickableBundle};
use bevy_prototype_lyon::prelude::tess::{geom::CubicBezierSegment, math::Point};
use rand::prelude::*;

use super::*;

#[derive(Component)]
pub struct TrainGhost;

pub fn draw_train_ghost(commands: &mut Commands, pos: Vec2, color: Color) {
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
pub fn nearest_on_curve(curve: CubicBezierSegment<f32>, point: Point) -> f32 {
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

    // Attempt to get more accurate via projection, very flaky, clamp first
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

    base.clamp(0.0, 1.0)
}

// Returns track id, t, point, distance
pub fn find_nearest_track(
    network: &Network,
    point: Point,
    cutoff: f32,
) -> Option<(TrackID, f32, Point, f32)> {
    let filtered = network.tracks.iter().filter_map(|(id, track)| {
        let line = track.curve.baseline();
        let projected = project_onto_line(line.from, line.to, point);
        let dist = point.distance_to(projected);
        if dist < cutoff + 200. {
            Some((*id, track))
        } else {
            None
        }
    });
    let refined = filtered.filter_map(|(id, track)| {
        let sample = nearest_on_curve(track.curve, point);
        let nearest = track.curve.sample(sample);
        let distance = point.distance_to(nearest);
        if distance > cutoff {
            None
        } else {
            Some((id, sample, nearest, distance))
        }
    });

    refined.min_by_key(|(_, _, _, distance)| FloatOrd(*distance))
}

pub struct TrainPlacementEvent {
    track: TrackID,
    sample: f32,
    shift: bool,
}

pub fn train_placement_tool(
    mut commands: Commands,
    network: Res<Network>,
    mouse_pos: Res<MousePos>,
    train: Query<Entity, With<TrainGhost>>,
    mouse_buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    mut writer: EventWriter<TrainPlacementEvent>,
) {
    train.for_each(|e| commands.entity(e).despawn());
    let mouse_pos = match mouse_pos.0 {
        Some(pos) => pos,
        None => return,
    };
    let mouse_point = Point::new(mouse_pos.x, mouse_pos.y);
    let nearest = find_nearest_track(network.as_ref(), mouse_point, 100.);

    if let Some((track, sample, point, _)) = nearest {
        draw_train_ghost(
            &mut commands,
            Vec2::new(point.x, point.y),
            Color::rgba(0.0, 0.0, 1.0, 0.95),
        );
        if mouse_buttons.just_pressed(MouseButton::Left) {
            let shift = keys.any_pressed([KeyCode::LShift, KeyCode::RShift]);
            writer.send(TrainPlacementEvent {
                track,
                sample,
                shift,
            })
        }
    }
}

pub fn cleanup_train_placement(mut commands: Commands, ghosts: Query<Entity, With<TrainGhost>>) {
    ghosts.for_each(|g| commands.entity(g).despawn());
}

#[derive(Component)]
pub struct Train {
    track_edge: TrackEdge,
    sample: f32,
    speed: f32,
}

impl Train {
    pub fn direction(&self) -> TrackDirection {
        self.track_edge.direction
    }

    pub fn flip(&mut self) {
        self.sample = 1. - self.sample;
        self.track_edge.direction = self.track_edge.direction.inverse();
        self.speed = -self.speed;
    }
}

pub fn place_train(
    mut commands: Commands,
    mut events: EventReader<TrainPlacementEvent>,
    network: Res<Network>,
) {
    for event in events.iter() {
        let track = network.get(event.track);
        if let Some(track) = track {
            let point = track.curve.sample(event.sample);
            let pos = Vec2::new(point.x, point.y);

            let circle = shapes::Circle {
                radius: 16.,
                ..default()
            };

            let mut ec = commands.spawn_bundle(GeometryBuilder::build_as(
                &circle,
                DrawMode::Fill(FillMode::color(Color::BLUE)),
                Transform::from_translation(pos.extend(20.)),
            ));

            ec.insert_bundle(PickableBundle::default()).insert(Train {
                track_edge: TrackEdge::pos(event.track),
                sample: event.sample,
                speed: 0.,
            });
            if !event.shift {
                ec.insert(Driving(TrackDirection::POS));
            }
        }
    }
}

#[derive(Component)]
pub struct Driving(TrackDirection);

fn move_along(track: &TrackData, train: &mut Train, amount: f32) -> f32 {
    let scaled = amount / track.length;
    let sample = (train.sample + scaled).clamp(0.0, 1.0);
    let delta = (sample - train.sample) * track.length;

    train.sample = sample;

    delta
}

fn update_train<F>(
    train: &mut Train,
    tf: &mut Transform,
    network: &Network,
    delta: f32,
    mut choose_track: F,
) where
    F: FnMut(&[(&TrackEdge, &TrackData)]) -> usize,
{
    let data = network.get_data(train.track_edge);
    if let Some(mut track_data) = data {
        let mut speed = train.speed * delta;
        while speed > 0. {
            if train.sample >= 1.0 {
                let node = track_data.get_pos(train.direction());
                let nodes = network.get_exits(&node);

                if !nodes.is_empty() {
                    let (edge, next) = nodes[choose_track(&nodes[..])];
                    train.sample = 0.;
                    train.track_edge = *edge;
                    track_data = next;
                } else {
                    train.speed = 0.
                }
            }

            let delta = move_along(track_data, train, speed);
            if delta == 0. {
                break;
            }
            speed -= delta;
        }

        let point = if train.direction().is_pos() {
            track_data.curve.sample(train.sample)
        } else {
            track_data.curve.sample(1. - train.sample)
        };

        tf.translation.x = point.x;
        tf.translation.y = point.y;
    }
}

const TRAIN_ACC: f32 = 200.;
pub fn drive_trains(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    network: Res<Network>,
    mut trains: Query<(&mut Train, &mut Transform, &mut Driving)>,
) {
    trains.for_each_mut(|(mut train, mut tf, mut driving)| {
        if keys.pressed(KeyCode::W) {
            train.speed =
                (train.speed + time.delta_seconds() * TRAIN_ACC * driving.0.signum()).clamp(-300., 300.);
        }
        if keys.pressed(KeyCode::S) {
            train.speed =
                (train.speed - time.delta_seconds() * TRAIN_ACC * driving.0.signum()).clamp(-300., 300.);
        }
        if train.speed < 0. {
            train.flip();
            driving.0 = driving.0.inverse();
        }

        let track_data = network.get_data(train.track_edge);
        let curr_direction = train.direction();
        if let Some(track_data) = track_data {
            let curr_end = track_data.get_pos(curr_direction);
            let end_vec = IVec2::from(curr_end.tile).as_vec2();
            update_train(
                &mut train,
                &mut tf,
                network.as_ref(),
                time.delta_seconds(),
                |exits| {
                    let end = track_data.get_pos(curr_direction);
                    let mut facing = end.facing.inverse();

                    let left = keys.pressed(KeyCode::A);
                    let right = keys.pressed(KeyCode::D);

                    if left {
                        facing = facing.perp().inverse();
                    }
                    if right {
                        facing = facing.perp();
                    }

                    if !driving.0.is_pos() && (left || right) {
                        facing = facing.inverse();
                    }

                    let target_vec = octant_to_unit(facing);

                    let (index, _) = exits
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, (edge, data))| {
                            let tile_pos = data.get_pos(edge.direction);
                            let tile_vec = IVec2::from(tile_pos.tile).as_vec2();
                            let vec = tile_vec - end_vec;
                            FloatOrd(vec.angle_between(target_vec).abs())
                        })
                        .unwrap();

                    index
                },
            );
        }
    });
}

pub fn update_trains(
    time: Res<Time>,
    network: Res<Network>,
    mut rand: ResMut<StdRng>,
    mut trains: Query<(&mut Train, &mut Transform), Without<Driving>>,
) {
    trains.for_each_mut(|(mut train, mut tf)| {
        train.speed = (train.speed + time.delta_seconds() * TRAIN_ACC).min(300.);
        update_train(
            &mut train,
            &mut tf,
            network.as_ref(),
            time.delta_seconds(),
            |exits| rand.gen_range(0..exits.len()),
        );
    });
}

pub fn remove_trains(
    mut commands: Commands,
    trains: Query<(Entity, &Hover), With<Train>>,
    mouse_buttons: Res<Input<MouseButton>>,
) {
    if mouse_buttons.pressed(MouseButton::Right) {
        trains.for_each(|(e, h)| {
            if h.hovered() {
                commands.entity(e).despawn();
            }
        });
    }
}
