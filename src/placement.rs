use std::{collections::VecDeque, f32::consts::PI};

use bevy::{prelude::*, utils::HashSet};
use bevy_egui::{egui, EguiContext};

use super::*;

fn ur_neighbours(tile: TilePos) -> Vec<TilePos> {
    vec![(tile.0, tile.1 + 1), (tile.0 + 1, tile.1)]
}

fn urd_neighbours(tile: TilePos) -> Vec<TilePos> {
    vec![
        (tile.0, tile.1 + 1),
        (tile.0 + 1, tile.1),
        (tile.0 + 1, tile.1 + 1),
    ]
}

#[derive(Debug)]
pub struct TrackSet {
    params: TrackParams,
    bends: Vec<TilePos>,
    diags: Vec<TilePos>,
    turn: TilePos,
}

impl TrackSet {
    fn new(params: TrackParams) -> Self {
        let (bends, diags, turn) = Self::generate_tracks(params.radius, params.divisor);

        Self {
            params,
            bends,
            diags,
            turn,
        }
    }

    fn generate_tracks(radius: f32, divisor: f32) -> (Vec<TilePos>, Vec<TilePos>, TilePos) {
        let (bends, turns) = Self::generate_bends(radius, divisor);
        let diags = Self::generate_diags(radius, divisor);
        (bends, diags, turns)
    }

    fn generate_bends(radius: f32, divisor: f32) -> (Vec<TilePos>, TilePos) {
        let facing = Vec2::Y;
        let start_pos = tile_to_center_pos((0, 0));
        let mut seen = HashSet::<TilePos>::new();
        let mut queue = VecDeque::new();
        let mut edge = Vec::new();

        seen.insert((0, 0));
        queue.push_back((0, 0));

        while let Some(curr) = queue.pop_front() {
            for next in ur_neighbours(curr) {
                if seen.contains(&next) {
                    continue;
                }
                seen.insert(next);
                let pos = tile_to_center_pos(next);
                let vec = pos - start_pos;
                let angle = vec.angle_between(facing);
                if angle >= 0. && angle <= PI / divisor {
                    if vec.length() >= radius {
                        edge.push(next);
                    } else {
                        queue.push_back(next);
                    }
                }
            }
        }

        edge.sort_by_cached_key(|pos| pos.0);
        let turn = edge.pop().unwrap();
        let turn = (turn.0 - 1, turn.1 - 1);
        (edge, turn)
    }

    fn generate_diags(radius: f32, divisor: f32) -> Vec<TilePos> {
        let facing = angle_to_unit(PI / 4.);
        let start_pos = tile_to_center_pos((0, 0));
        let mut seen = HashSet::<TilePos>::new();
        let mut queue = VecDeque::new();
        let mut edge = Vec::new();

        seen.insert((0, 0));
        queue.push_back((0, 0));

        while let Some(curr) = queue.pop_front() {
            for next in urd_neighbours(curr) {
                if seen.contains(&next) {
                    continue;
                }
                seen.insert(next);
                let pos = tile_to_center_pos(next);
                let vec = pos - start_pos;
                let angle = vec.angle_between(facing);
                if angle >= 0. && angle <= PI / divisor / 2. {
                    if vec.length() >= radius {
                        edge.push(next);
                    } else {
                        queue.push_back(next);
                    }
                }
            }
        }

        edge.sort_by_cached_key(|pos| pos.1);
        edge
    }

    fn draw(&self, path: &mut PathBuilder) {
        let zero = tile_to_center_pos((0, 0));

        for bend in &self.bends {
            path.move_to(zero);
            let end_pos = tile_to_center_pos(*bend);
            let ctrl_mag = zero.distance(end_pos) / 3.;
            path.cubic_bezier_to(
                zero + Vec2::Y * ctrl_mag,
                end_pos - Vec2::Y * ctrl_mag,
                end_pos,
            );
        }

        let up_diag = angle_to_unit(PI / 4.);
        let down_diag = angle_to_unit(PI + PI / 4.);

        for diag in &self.diags {
            path.move_to(zero);
            let end_pos = tile_to_center_pos(*diag);
            let ctrl_mag = zero.distance(end_pos) / 3.;
            path.cubic_bezier_to(
                zero + up_diag * ctrl_mag,
                end_pos + down_diag * ctrl_mag,
                end_pos,
            );
        }

        path.move_to(zero);
        let end_pos = tile_to_center_pos(self.turn);
        let ctrl_mag = zero.distance(end_pos) / 3.;
        path.cubic_bezier_to(
            zero + Vec2::Y * ctrl_mag,
            end_pos + down_diag * ctrl_mag,
            end_pos,
        );

        path.move_to(zero);
        let end_pos = tile_to_center_pos((self.turn.1, self.turn.0));
        let ctrl_mag = zero.distance(end_pos) / 3.;
        path.cubic_bezier_to(
            zero + up_diag * ctrl_mag,
            end_pos - Vec2::X * ctrl_mag,
            end_pos,
        );
    }
}

#[derive(Component)]
pub struct PlacementGhost;

// Octant orientation, 0 is north
type Orientation = i32;

pub struct PlacementStart(pub Option<TilePos>);

// fn place_next(
//     tracks: &mut Vec<(TilePos, Orientation)>,
//     start_tile: TilePos,
//     start_facing: Orientation,
//     towards: TilePos,
//     radius: f32,
// ) {
//     tracks.push((start_tile, start_facing));

//     if start_tile == towards {
//         return;
//     }

//     // Get tile that follows ray
//     let start_pos = tile_to_center_pos(start_tile);
//     let target_pos = tile_to_center_pos(towards);
//     let clamped_vec = (target_pos - start_pos).normalize_or_zero() * radius;
//     let end_tile = world_pos_to_tile(start_pos + clamped_vec);
//     let end_pos = tile_to_center_pos(end_tile);

//     // Determine if this falls within the correct orientation
//     let end_vec = end_pos - start_pos;
//     let end_facing = vec_to_octant(end_vec);

//     let angle = octant_to_angle(start_facing);
//     let left_bound = angle - PI / 8.;
//     let left_vec = angle_to_unit(left_bound) * radius;
//     let left_vec = left_vec.project_onto_normalized(angle_to_unit(angle + PI / 2.));

//     let offset_left = end_vec - left_vec;
//     println!("{:?}, {:?}", end_vec, left_vec);

//     let right_bound = angle + PI / 8.;
//     let right_vec = angle_to_unit(right_bound) * radius;

//     let diff = (((end_facing - start_facing) + 4) % 8) - 4;
//     if diff != 0 {
//         let angle = octant_to_angle(start_facing) + PI / 8. * diff.signum() as f32;
//         let ray_vec = angle_to_unit(angle) * radius;
//         let end_tile = world_pos_to_tile(start_pos + ray_vec);

//         let end_pos = tile_to_center_pos(end_tile);
//         let end_facing = vec_to_octant(end_pos - start_pos);

//         // Modify end tile to fall outside ray
//         if end_facing == start_facing {}

//         // path.move_to(start_pos);
//         // path.line_to(start_pos + ray_vec);

//         // place_next(tracks, end_tile, end_facing, towards, radius);
//         tracks.push((end_tile, end_facing));
//     } else {
//         tracks.push((end_tile, end_facing));

//         // place_next(tracks, end_tile, end_facing, towards, radius);
//     }
// }

// fn track_placement(
//     _path: &mut PathBuilder,
//     start_tile: TilePos,
//     start_facing: Orientation,
//     towards: TilePos,
//     radius: f32,
// ) -> Vec<(TilePos, Orientation)> {
//     let mut tracks = Vec::new();
//     place_next(&mut tracks, start_tile, start_facing, towards, radius);
//     tracks
// }

fn octant_to_unit(octant: i32) -> Vec2 {
    let angle = octant_to_angle(octant);
    angle_to_unit(angle)
}

fn octant_to_angle(octant: i32) -> f32 {
    octant as f32 * PI / 4.
}

fn angle_to_unit(angle: f32) -> Vec2 {
    Vec2::new(f32::sin(angle), f32::cos(angle))
}

// fn vec_to_octant(vec: Vec2) -> i32 {
//     let angle = vec.angle_between(Vec2::Y) + PI / 8.;
//     let abs_angle = (angle + 2. * PI) % (2. * PI);
//     let octant = abs_angle / (PI / 4.);
//     octant as i32
// }

// fn track_path(path: &mut PathBuilder, start: (TilePos, Orientation), end: (TilePos, Orientation)) {
//     let (start_tile, start_octant) = start;
//     let (end_tile, end_octant) = end;

//     let start_pos = tile_to_center_pos(start_tile);
//     let end_pos = tile_to_center_pos(end_tile);

//     let ctrl_mag = start_pos.distance(end_pos) / 3.;
//     path.move_to(start_pos);

//     let ctrl_one = octant_to_unit(start_octant) * ctrl_mag;
//     let ctrl_two = octant_to_unit(end_octant + 4) * ctrl_mag;
//     path.cubic_bezier_to(start_pos + ctrl_one, end_pos + ctrl_two, end_pos);
// }

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TrackParams {
    radius: f32,
    divisor: f32,
}

fn in_direction(start: TilePos, facing: Orientation, end: TilePos) -> bool {
    let dir = octant_to_unit(facing);
    Vec2::new((end.0 - start.0) as f32, (end.1 - start.1) as f32)
        .normalize_or_zero()
        .abs_diff_eq(dir, 0.01)
}

fn rotate_vec(vec: Vec2, angle: f32) -> Vec2 {
    Vec2::new(
        vec.x * angle.cos() - vec.y * angle.sin(),
        vec.x * angle.sin() + vec.y * angle.cos(),
    )
}

impl TrackParams {
    fn base_turn(&self) -> TilePos {
        let center = Vec2::new(self.radius, 0.);
        let offset = angle_to_unit(-PI / 4.) * self.radius;
        world_pos_to_tile(offset + center)
    }

    fn base_turn_vec(&self) -> Vec2 {
        let turn = self.base_turn();
        Vec2::new(turn.0 as f32 * TILE_SIZE, turn.1 as f32 * TILE_SIZE)
    }

    fn turn(&self, start: Vec2, facing: Orientation, direction: f32) -> TilePos {
        let mut vec = self.base_turn_vec();
        if direction < 0. {
            vec.x = -vec.x;
        }
        let vec = rotate_vec(vec, -octant_to_angle(facing));
        println!("{:?}, {:?}, {:?}", start, facing, vec);
        world_pos_to_tile(start + vec)
    }

    fn place_tracks(
        &self,
        start: (TilePos, Orientation),
        end_tile: TilePos,
    ) -> Vec<(TilePos, Orientation)> {
        let mut tracks = Vec::new();

        let (start_tile, start_facing) = start;
        tracks.push((start_tile, start_facing));

        let start_vec = octant_to_unit(start_facing);
        let start_pos = tile_to_center_pos(start_tile);

        let end_pos = tile_to_center_pos(end_tile);
        let end_vec = end_pos - start_pos;
        let angle = end_vec.angle_between(start_vec);

        // Simple case straight
        if in_direction(start_tile, start_facing, end_tile) {
            tracks.push((end_tile, start_facing));
        }
        // Simple case s bend
        else if angle.abs() <= PI / self.divisor && end_vec.length() <= self.radius {
            tracks.push((end_tile, start_facing));
        // Straight until bend would face target
        } else {
            let turn = self.base_turn();
            let upto = end_vec.project_onto_normalized(start_vec);
            let dist = upto.distance(end_vec);
            // Height of curve + height gained on diagonal to target
            let turn_height = turn.1 as f32 * TILE_SIZE + dist / octant_to_angle(1).tan();
            let mut turn_from = start_pos;

            if upto.length() > turn_height && upto.dot(start_vec) > 0. {
                let upto = upto - upto.normalize() * turn_height;
                turn_from = start_pos + upto;
                tracks.push((world_pos_to_tile(turn_from), start_facing));
            }

            let turn_end = self.turn(turn_from, start_facing, angle.signum());
            let target_facing = start_facing + angle.signum() as i32;
            tracks.push((turn_end, target_facing));
        }

        tracks
    }
}

pub fn setup_placement(mut commands: Commands) {
    let params = TrackParams {
        radius: 240.,
        divisor: 5.8,
    };
    let set = TrackSet::new(params);
    println!("{:?}", set);

    commands.insert_resource(set);
    commands.insert_resource(params)
}

pub fn placement(
    mut facing: Local<Orientation>,
    mut commands: Commands,
    mut start: ResMut<PlacementStart>,
    tracks: Res<TrackSet>,
    params: Res<TrackParams>,
    mouse_pos: Res<MousePos>,
    mouse_buttons: Res<Input<MouseButton>>,
    ghosts: Query<Entity, With<PlacementGhost>>,
) {
    if mouse_buttons.just_pressed(MouseButton::Middle) {
        *facing = *facing + 1;
    }
    ghosts.for_each(|e| commands.entity(e).despawn());
    if let Some(mouse_pos) = mouse_pos.0 {
        let mouse_tile = world_pos_to_tile(mouse_pos);
        if let None = start.0 && mouse_buttons.just_pressed(MouseButton::Left) {
            start.0 = Some(mouse_tile);
        } else if let Some(start_tile) = start.0 {
            // let mut path = PathBuilder::new();
            // tracks.draw(&mut path);
            // let bundle = GeometryBuilder::build_as(
            //     &path.build(),
            //     DrawMode::Stroke(StrokeMode::new(Color::rgba(0.5, 0.5, 0.5, 0.5), 4.)),
            //     Transform::default(),
            // );
            // commands.spawn_bundle(bundle).insert(PlacementGhost);

            let mut path = PathBuilder::new();
            let tracks = params.place_tracks((start_tile, *facing), mouse_tile);
            for pair in tracks.windows(2) {
                track_path(&mut path, pair[0], pair[1]);
            }
            let bundle = GeometryBuilder::build_as(
                &path.build(),
                DrawMode::Stroke(StrokeMode::new(Color::WHITE, 6.)),
                Transform::default(),
            );
            commands.spawn_bundle(bundle).insert(PlacementGhost);
        }
    }
}

fn track_path(path: &mut PathBuilder, start: (TilePos, Orientation), end: (TilePos, Orientation)) {
    let (start_tile, start_facing) = start;
    let (end_tile, end_facing) = end;
    let start_pos = tile_to_center_pos(start_tile);
    let end_pos = tile_to_center_pos(end_tile);

    path.move_to(start_pos);

    let ctrl_mag = start_pos.distance(end_pos) / 3.;
    let start_ctrl = octant_to_unit(start_facing);
    let end_ctrl = octant_to_unit(end_facing + 4);
    path.cubic_bezier_to(
        start_pos + start_ctrl * ctrl_mag,
        end_pos + end_ctrl * ctrl_mag,
        end_pos,
    );
}

fn track_placement(
    tracks: &TrackSet,
    start_tile: TilePos,
    octant: Orientation,
    mouse_tile: TilePos,
) -> Vec<(TilePos, Orientation)> {
    todo!()
}

pub fn track_control(
    mut ctx: ResMut<EguiContext>,
    mut tracks: ResMut<TrackSet>,
    mut params: ResMut<TrackParams>,
) {
    egui::Window::new("Tracks").show(ctx.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut params.radius, 100.0..=1000.0).text("Radius"));
        ui.add(egui::Slider::new(&mut params.divisor, 1.0..=12.0).text("Angle"));
    });

    if *params != tracks.params {
        *tracks = TrackSet::new(*params);
    }
}
