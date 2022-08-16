use bevy::prelude::*;
use bevy_prototype_lyon::entity::ShapeBundle;
use petgraph::prelude::UnGraphMap;

use super::*;

#[derive(Component)]
pub struct Network {
    track_graph: UnGraphMap<TilePos, ()>,
}

pub fn setup_network(mut commands: Commands) {
    commands
        .spawn()
        .insert(Network {
            track_graph: UnGraphMap::from_edges(&[
                ((5, 5), (5, 7)),
                ((5, 7), (7, 7)),
                ((5, 7), (3, 7)),
                ((7, 7), (8, 8)),
            ]),
        })
        .insert_bundle(ShapeBundle {
            mode: DrawMode::Stroke(StrokeMode {
                options: StrokeOptions::default()
                    .with_line_cap(LineCap::Round)
                    .with_line_width(10.0),
                color: Color::WHITE,
            }),
            ..default()
        });
}

pub fn extract_network_to_mesh(mut networks: Query<(&mut Network, &mut Path), Changed<Network>>) {
    networks.for_each_mut(|(network, mut path)| {
        let mut path_builder = PathBuilder::new();
        network.track_graph.all_edges().for_each(|edge| {
            path_builder.move_to(tile_to_world_pos(edge.0) + TILE_SIZE / 2.);
            path_builder.line_to(tile_to_world_pos(edge.1) + TILE_SIZE / 2.);
        });
        *path = path_builder.build();
    });
}
