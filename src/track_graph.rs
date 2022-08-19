use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_prototype_lyon::prelude::tess::geom::{CubicBezierSegment, Point};
use petgraph::prelude::DiGraphMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

pub type TrackID = usize;
static NEXT_TRACK_ID: AtomicUsize = AtomicUsize::new(0);

pub struct TrackData {
    pub segment: TrackSegment,
    pub curve: CubicBezierSegment<f32>,
}

impl TrackData {
    pub fn start_tile(&self) -> TileIndex {
        self.segment.start.tile
    }

    pub fn end_tile(&self) -> TileIndex {
        self.segment.end.tile
    }
}

impl From<TrackSegment> for TrackData {
    fn from(segment: TrackSegment) -> Self {
        let (start, ctrl1, ctrl2, end) = segment.control_points();

        Self {
            segment,
            curve: CubicBezierSegment {
                from: Point::new(start.x, start.y),
                ctrl1: Point::new(ctrl1.x, ctrl1.y),
                ctrl2: Point::new(ctrl2.x, ctrl2.y),
                to: Point::new(end.x, end.y),
            },
        }
    }
}

#[derive(Default)]
pub struct Network {
    pathing_graph: DiGraphMap<TrackPos, TrackID>,
    // track_graph: UnGraphMap<TileIndex, TrackEdge>,
    pub tracks: HashMap<TrackID, TrackData>,
}

impl Network {
    pub fn get_connections(&self, tile: TileIndex) -> [bool; 8] {
        let mut exists = [false; 8];
        for o in 0..8 {
            let node = TrackPos {
                facing: o.into(),
                tile,
            };
            exists[o] = self.pathing_graph.contains_node(node);
        }
        exists
    }
}

#[derive(Clone, Copy, Default, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct TrackEdge {
    pub start: Octant,
    pub end: Octant,
}

impl From<&TrackSegment> for TrackEdge {
    fn from(segment: &TrackSegment) -> Self {
        Self {
            start: segment.start.facing,
            end: segment.end.facing,
        }
    }
}

pub fn setup_network(mut commands: Commands) {
    commands.insert_resource(Network::default());
}

pub struct NetworkRenderEvent;

#[derive(Component)]
pub struct NetworkTrack(pub TrackID);

#[derive(Component)]
pub struct NetworkNode(pub TileIndex);

pub fn extract_network_to_mesh(
    mut commands: Commands,
    network: Res<Network>,
    events: EventReader<NetworkRenderEvent>,
    tracks: Query<Entity, With<NetworkTrack>>,
    nodes: Query<Entity, With<NetworkNode>>,
) {
    if !events.is_empty() {
        tracks.for_each(|e| commands.entity(e).despawn());
        nodes.for_each(|e| commands.entity(e).despawn());

        let mut nodes = HashSet::new();
        network.tracks.iter().for_each(|(id, track)| {
            draw_track(&mut commands, *id, &track.segment);
            nodes.insert(track.start_tile());
            nodes.insert(track.end_tile());
        });

        nodes.iter().for_each(|node| {
            draw_node(&mut commands, *node);
        });
    }
}

pub struct TrackPlacementEvent(pub TrackSegment);

pub fn place_tracks(
    mut events: EventReader<TrackPlacementEvent>,
    mut network: ResMut<Network>,
    mut render: EventWriter<NetworkRenderEvent>,
) {
    for TrackPlacementEvent(segment) in events.iter() {
        let id = NEXT_TRACK_ID.fetch_add(1, Ordering::SeqCst);
        network
            .pathing_graph
            .add_edge(segment.start, segment.end.inverse(), id);
        network
            .pathing_graph
            .add_edge(segment.end, segment.start.inverse(), id);

        network.tracks.insert(id, TrackData::from(*segment));
        // network.track_graph.add_edge(
        //     segment.start.tile,
        //     segment.end.tile,
        //     TrackEdge::from(segment),
        // );
        render.send(NetworkRenderEvent);
    }
}
