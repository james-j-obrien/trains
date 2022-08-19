use bevy_mod_picking::PickableBundle;
use bevy_prototype_lyon::entity::ShapeBundle;

use super::*;

pub fn draw_track(commands: &mut Commands, id: TrackID, track: &TrackSegment) {
    let mut path_builder = PathBuilder::new();
    track_path(&mut path_builder, track);

    commands
        .spawn_bundle(build_path(path_builder, Color::WHITE, 8., 10.))
        .insert_bundle(PickableBundle::default())
        .insert(NetworkTrack(id));
}

pub fn draw_node(commands: &mut Commands, node: TileIndex) {
    let circle = shapes::Circle {
        radius: 8.,
        center: tile_to_center(node),
    };

    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &circle,
            DrawMode::Stroke(StrokeMode {
                color: Color::WHITE,
                options: StrokeOptions::default().with_line_width(3.),
            }),
            Transform::default(),
        ))
        .insert(NetworkNode(node));
}

pub fn track_path(path: &mut PathBuilder, track: &TrackSegment) {
    let (start, ctrl_one, ctrl_two, end) = track.control_points();
    path.move_to(start);
    path.cubic_bezier_to(ctrl_one, ctrl_two, end);
}

pub fn build_path(path: PathBuilder, color: Color, width: f32, z: f32) -> ShapeBundle {
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
