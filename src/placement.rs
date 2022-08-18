use std::{f32::consts::PI, ops::Index};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use bevy_prototype_lyon::entity::ShapeBundle;

use super::*;

#[derive(Component)]
pub struct PlacementGhost;

// Octant orientation, 0 is north
type Orientation = i8;

#[derive(Default)]
pub struct PlacementState {
    start: Option<TilePos>,
    facing_options: [bool; 8],
    facing: Option<Orientation>,
}

fn octant_to_unit(octant: Orientation) -> Vec2 {
    let angle = octant_to_angle(octant);
    angle_to_unit(angle)
}

fn octant_to_angle(octant: Orientation) -> f32 {
    octant as f32 * PI / 4.
}

fn angle_to_unit(angle: f32) -> Vec2 {
    Vec2::new(f32::sin(angle), f32::cos(angle))
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TrackParams {
    radius: f32,
}

fn in_direction(start: Vec2, facing: Orientation, end: Vec2) -> bool {
    let dir = octant_to_unit(facing);
    (end - start).normalize_or_zero().abs_diff_eq(dir, 0.01)
}

impl TrackParams {
    fn get_turn(&self, facing: Orientation, dir: f32) -> Vec2 {
        let dir = -dir.signum();
        let unit = octant_to_unit(facing);
        let center = unit.perp() * self.radius * dir;
        let offset = octant_to_unit(facing + dir as Orientation) * self.radius;
        (offset + center).round()
    }

    fn place_tracks(
        &self,
        start_tile: TilePos,
        start_facing: Orientation,
        target_tile: TilePos,
        allow_bends: bool,
    ) -> Vec<(TilePos, Orientation)> {
        let mut tracks = Vec::new();
        tracks.push((start_tile, start_facing));

        let start_vec = start_tile.as_vec2();
        let target_vec = target_tile.as_vec2();

        // Simple case straight
        if in_direction(start_vec, start_facing, target_vec) {
            tracks.push((target_tile, start_facing));
            return tracks;
        }

        let start_unit = octant_to_unit(start_facing);
        let tile_vec = target_vec - start_vec;
        let tile_angle = tile_vec.angle_between(start_unit);

        let turn_vec = self.get_turn(start_facing, tile_angle);
        let turn_angle = start_unit.angle_between(turn_vec);
        let abs_turn_angle = turn_angle.abs();

        // Calculate bend along the vector projected by the turn
        let projected_tile = tile_vec.project_onto_normalized(start_unit.perp());
        let projected_turn = turn_vec.project_onto_normalized(start_unit.perp());
        let ratio = projected_tile.length() / projected_turn.length();
        let bend = (ratio * turn_vec).round();

        let straight_vec = (tile_vec - bend).round();
        let straight_tile = straight_vec.round().as_ivec2();
        let can_bend =
            allow_bends && (straight_tile == IVec2::ZERO || straight_vec.dot(start_unit) > 0.);

        // Bend, don't allow if could be replaced by two turns or if too sharp
        if can_bend && bend.length_squared() <= (turn_vec * 2.).length_squared() {
            let straight_vec = (tile_vec - bend).round().as_ivec2();
            if straight_vec != IVec2::ZERO {
                tracks.push((start_tile + straight_vec, start_facing));
            }
            tracks.push((start_tile + straight_vec + bend.as_ivec2(), start_facing));
            // Straight until turn would face target
        } else {
            // Project onto start_vec instead of perp
            let projected_tile = tile_vec.project_onto_normalized(start_unit);
            let projected_dist = projected_tile.distance(tile_vec);

            let turn_length = turn_vec.length();

            let straight_length = projected_tile.length() - projected_dist;
            // let straight_vec = start_unit * straight_length;
            // let diag_length = straight_vec.distance(tile_vec);

            let turn_straight_length =
                turn_length * (PI / 4. - abs_turn_angle).sin() / (0.75 * PI).sin();
            let straight_length = straight_length - turn_straight_length;

            // let target_unit = octant_to_unit(target_facing);
            // let turn_diag_length = turn_length * abs_turn_angle.sin() / (0.75 * PI).sin();
            // let diag_length = diag_length - turn_diag_length;
            // let diag_vec = target_unit * diag_length;

            let straight_vec = start_unit * straight_length;
            let straight_tile = straight_vec.round().as_ivec2();

            let turn_tile = turn_vec.as_ivec2();
            let target_facing = start_facing + tile_angle.signum() as Orientation;

            if straight_tile == IVec2::ZERO
                || straight_length < 0.
                || projected_tile.dot(start_unit) < 0.
            {
                tracks.push((start_tile + turn_tile, target_facing));
            } else {
                tracks.push((start_tile + straight_tile, start_facing));
                tracks.push((start_tile + straight_tile + turn_tile, target_facing));
            };
        }

        tracks
    }
}

#[derive(Component)]
pub struct Arrow;

#[derive(Component)]
pub struct ArrowHighlighter {
    arrows: [Entity; 8],
}

impl ArrowHighlighter {
    const HEIGHT: f32 = 10.;
    const HIGHLIGHT_COLOR: Color = Color::GRAY;
    const NORMAL_COLOR: Color = Color::DARK_GRAY;

    fn spawn(commands: &mut Commands) {
        let mut children = [Entity::from_bits(0); 8];
        commands
            .spawn_bundle(SpatialBundle {
                visibility: Visibility { is_visible: false },
                ..default()
            })
            .with_children(|parent| {
                let triangle = shapes::RegularPolygon {
                    sides: 3,
                    feature: shapes::RegularPolygonFeature::SideLength(TILE_SIZE / 2.),
                    ..shapes::RegularPolygon::default()
                };

                for i in 0..8 {
                    let unit = octant_to_unit(i) * TILE_SIZE;
                    let angle = octant_to_angle(i);

                    let id = parent
                        .spawn_bundle(GeometryBuilder::build_as(
                            &triangle,
                            DrawMode::Fill(FillMode::color(Color::GRAY)),
                            Transform::from_xyz(unit.x, unit.y, Self::HEIGHT)
                                .with_rotation(Quat::from_rotation_z(-angle)),
                        ))
                        .insert(Arrow)
                        .id();

                    children[i as usize] = id;
                }
            })
            .insert(ArrowHighlighter { arrows: children });
    }
}

impl Index<usize> for ArrowHighlighter {
    type Output = Entity;

    fn index(&self, index: usize) -> &Self::Output {
        &self.arrows[index]
    }
}

pub fn setup_placement(mut commands: Commands) {
    let params = TrackParams { radius: 6. };

    commands.insert_resource(params);
    ArrowHighlighter::spawn(&mut commands);
}

fn build_path(path: PathBuilder, color: Color, width: f32, z: f32) -> ShapeBundle {
    GeometryBuilder::build_as(
        &path.build(),
        DrawMode::Stroke(StrokeMode {
            options: StrokeOptions::default()
                .with_line_cap(LineCap::Round)
                .with_line_width(width),
            color,
        }),
        Transform::from_xyz(0., 0., z),
    )
}

pub fn placement(
    mut commands: Commands,
    mut placement: ResMut<PlacementState>,
    mut arrow_highlighter: Query<
        (&mut Transform, &mut Visibility, &ArrowHighlighter),
        Without<Arrow>,
    >,
    mut arrows: Query<(&mut Visibility, &mut DrawMode), With<Arrow>>,

    params: Res<TrackParams>,
    mouse_pos: Res<MousePos>,
    mouse_buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    ghosts: Query<Entity, With<PlacementGhost>>,
) {
    let allow_bends = keys.any_pressed([KeyCode::LShift, KeyCode::RShift]);
    let (mut arrows_tf, mut arrows_vis, arrow_highlighter) = arrow_highlighter.single_mut();

    ghosts.for_each(|e| commands.entity(e).despawn());
    if let Some(mouse_pos) = mouse_pos.0 {
        if mouse_buttons.just_pressed(MouseButton::Right) {
            placement.start = None;
            placement.facing = None;
            arrows_vis.is_visible = false;
        }

        let mouse_tile = world_pos_to_tile(mouse_pos);
        match (placement.start, placement.facing) {
            (None, _) => {
                if mouse_buttons.just_pressed(MouseButton::Left) {
                    placement.start = Some(mouse_tile);

                    // TODO
                    placement.facing_options = [true; 8];

                    let tile_pos = tile_to_center_pos(mouse_tile);
                    arrows_tf.translation.x = tile_pos.x;
                    arrows_tf.translation.y = tile_pos.y;
                    arrows_vis.is_visible = true;
                }
            }
            (Some(start_tile), None) => {
                let start_pos = tile_to_center_pos(start_tile);
                let mouse_vec = mouse_pos - start_pos;

                // Get closest direction to mouse_angle
                let (best, _) = placement.facing_options.iter().enumerate().fold(
                    (0, f32::INFINITY),
                    |(best, to_beat), (index, flag)| {
                        let unit = octant_to_unit(index as Orientation);
                        let diff = mouse_vec.normalize_or_zero().distance_squared(unit);
                        if *flag && diff < to_beat {
                            (index, diff)
                        } else {
                            (best, to_beat)
                        }
                    },
                );

                let arrows = arrows
                    .get_many_mut(arrow_highlighter.arrows)
                    .expect("Highlighter arrow entities missing");

                arrows.into_iter().enumerate().for_each(|(i, arrow)| {
                    let (mut vis, mut dm) = arrow;
                    vis.is_visible = placement.facing_options[i];
                    if let DrawMode::Fill(FillMode { color, .. }) = dm.as_mut() {
                        *color = if best == i {
                            ArrowHighlighter::HIGHLIGHT_COLOR
                        } else {
                            ArrowHighlighter::NORMAL_COLOR
                        }
                    }
                });

                if mouse_buttons.just_pressed(MouseButton::Left) {
                    placement.facing = Some(best as Orientation);
                    arrows_vis.is_visible = false;
                }
            }
            (Some(start_tile), Some(facing)) => {
                let mut path = PathBuilder::new();
                let tracks = params.place_tracks(start_tile, facing, mouse_tile, allow_bends);
                for pair in tracks.windows(2) {
                    track_path(&mut path, pair[0], pair[1]);
                }
                commands
                    .spawn_bundle(build_path(path, Color::GRAY, 4., 0.1))
                    .insert(PlacementGhost);

                if mouse_buttons.just_pressed(MouseButton::Left) {
                    let mut path = PathBuilder::new();
                    track_path(&mut path, tracks[0], tracks[1]);
                    commands.spawn_bundle(build_path(path, Color::WHITE, 8., 0.4));

                    placement.start = Some(tracks[1].0);
                    placement.facing = Some(tracks[1].1);
                }
            }
        };
    }
}

const HASH_LENGTH: f32 = 15.;

fn track_path(path: &mut PathBuilder, start: (TilePos, Orientation), end: (TilePos, Orientation)) {
    let (start_tile, start_facing) = start;
    let (end_tile, end_facing) = end;
    let start_pos = tile_to_center_pos(start_tile);
    let end_pos = tile_to_center_pos(end_tile);

    let hash_vec = octant_to_unit(start_facing + 2);

    path.move_to(start_pos - hash_vec * HASH_LENGTH);
    path.line_to(start_pos + hash_vec * HASH_LENGTH);

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

pub fn track_control(mut ctx: ResMut<EguiContext>, mut params: ResMut<TrackParams>) {
    egui::Window::new("Tracks").show(ctx.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut params.radius, 2.5..=20.0).text("Radius"));
        ui.add_space(4.0);
        ui.label("Left-click to build tracks.");
        ui.label("Right-click to cancel.");
        ui.label("Hold Shift to allow S-bends.");
        ui.label("Left-click to build tracks.");
        ui.label("Scroll to zoom.");
        ui.label("Middle mouse to pan.");
    });
}
