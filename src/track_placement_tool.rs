use std::{f32::consts::PI, ops::Index};

use bevy::prelude::*;
use bevy_mod_picking::{HoverEvent, PickingEvent};

use super::*;

#[derive(Component)]
pub struct TrackGhost;

#[derive(Default)]
pub struct PlacementState {
    start: Option<TileIndex>,
    facing_options: [bool; 8],
    facing: Option<Octant>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TrackParams {
    pub radius: f32,
}

fn in_direction(start: Vec2, facing: Octant, end: Vec2) -> bool {
    let dir = octant_to_unit(facing);
    (end - start).normalize_or_zero().abs_diff_eq(dir, 0.01)
}

impl TrackParams {
    fn get_turn(&self, facing: Octant, dir: f32) -> Vec2 {
        let dir = -dir.signum();
        let unit = octant_to_unit(facing);
        let center = unit.perp() * self.radius * dir;
        let offset = octant_to_unit(facing + dir.into()) * self.radius;
        (offset + center).round()
    }

    fn place_tracks(
        &self,
        start_tile: TileIndex,
        start_facing: Octant,
        target_tile: TileIndex,
        allow_bends: bool,
    ) -> Vec<TrackPos> {
        let tracks = self.place_vec_tracks(
            tile_to_vec(start_tile),
            start_facing,
            tile_to_vec(target_tile),
            allow_bends,
        );
        let tracks: Vec<TrackPos> = tracks.into_iter().map(|t| t.into()).collect();
        tracks
    }

    fn place_vec_tracks(
        &self,
        start_tile: TileVec,
        start_facing: Octant,
        target_tile: TileVec,
        allow_bends: bool,
    ) -> Vec<(TileVec, Octant)> {
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
            let target_facing = start_facing + tile_angle.signum().into();

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
pub struct Track;

#[derive(Component)]
pub struct Arrow;

#[derive(Component)]
pub struct ArrowHighlighter {
    arrows: [Entity; 8],
}

impl ArrowHighlighter {
    const HEIGHT: f32 = 100.;
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

pub fn setup_track_placement(mut commands: Commands) {
    let params = TrackParams { radius: 6. };

    commands.insert_resource(params);
    ArrowHighlighter::spawn(&mut commands);
}

pub fn track_placement_tool(
    mut commands: Commands,
    mut placement: ResMut<PlacementState>,
    mut arrow_highlighter: Query<
        (&mut Transform, &mut Visibility, &ArrowHighlighter),
        Without<Arrow>,
    >,
    mut arrows: Query<(&mut Visibility, &mut DrawMode), With<Arrow>>,
    mut events: EventWriter<TrackPlacementEvent>,

    network: Res<Network>,
    params: Res<TrackParams>,
    mouse_pos: Res<MousePos>,
    mouse_buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    ghosts: Query<Entity, With<TrackGhost>>,
) {
    let shift = keys.any_pressed([KeyCode::LShift, KeyCode::RShift]);
    let (mut arrows_tf, mut arrows_vis, arrow_highlighter) = arrow_highlighter.single_mut();

    ghosts.for_each(|e| commands.entity(e).despawn());
    if let Some(mouse_pos) = mouse_pos.0 {
        if mouse_buttons.just_pressed(MouseButton::Right) {
            placement.start = None;
            placement.facing = None;
            arrows_vis.is_visible = false;
        }

        let mouse_tile = pos_to_tile(mouse_pos);
        match (placement.start, placement.facing) {
            (None, _) => {
                if mouse_buttons.just_pressed(MouseButton::Left) {
                    placement.start = Some(mouse_tile);

                    if shift {
                        placement.facing_options = [true; 8];
                    } else {
                        placement.facing_options = network.get_connections(mouse_tile);
                        if placement.facing_options.iter().all(|b| !b) {
                            placement.facing_options = [true; 8];
                        }
                    }

                    let tile_pos = tile_to_center(mouse_tile);
                    arrows_tf.translation.x = tile_pos.x;
                    arrows_tf.translation.y = tile_pos.y;
                }
            }
            (Some(start_tile), None) => {
                let start_pos = tile_to_center(start_tile);
                let mouse_vec = mouse_pos - start_pos;

                // Get closest direction to mouse_angle
                let (best, _) = placement.facing_options.iter().enumerate().fold(
                    (0, f32::INFINITY),
                    |(best, to_beat), (index, flag)| {
                        let unit = octant_to_unit(index);
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
                arrows_vis.is_visible = true;

                if mouse_buttons.just_pressed(MouseButton::Left) {
                    placement.facing = Some(best.into());
                    arrows_vis.is_visible = false;
                }
            }
            (Some(start_tile), Some(facing)) => {
                let mut path = PathBuilder::new();
                let tracks = params.place_tracks(start_tile, facing, mouse_tile, shift);
                let segments: Vec<TrackSegment> = tracks
                    .windows(2)
                    .map(|pair| TrackSegment::from_directed(pair[0], pair[1]))
                    .collect();
                for segment in segments.iter() {
                    track_path(&mut path, segment);
                }
                commands
                    .spawn_bundle(build_path(path, Color::GRAY, 4., 0.1))
                    .insert(TrackGhost);

                if mouse_buttons.just_pressed(MouseButton::Left) {
                    placement.start = Some(tracks[1].tile);
                    placement.facing = Some(tracks[1].facing);

                    events.send(TrackPlacementEvent(segments[0]));
                }
            }
        };
    }
}

pub fn cleanup_track_placement(
    mut commands: Commands,
    ghosts: Query<Entity, With<TrackGhost>>,
    mut placement: ResMut<PlacementState>,
    mut arrow_highlighter: Query<&mut Visibility, With<ArrowHighlighter>>,
) {
    arrow_highlighter.single_mut().is_visible = false;
    ghosts.for_each(|g| commands.entity(g).despawn());
    placement.facing = None;
    placement.start = None;
}
