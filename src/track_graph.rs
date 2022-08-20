use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_mod_picking::Hover;
use bevy_prototype_lyon::prelude::tess::geom::{CubicBezierSegment, Point};
use petgraph::prelude::DiGraphMap;
use std::collections::HashMap;
use std::ops::Mul;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

pub type TrackID = usize;
static NEXT_TRACK_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Default, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct TrackDirection(bool);
impl TrackDirection {
    pub const POS: TrackDirection = TrackDirection(true);
    pub const NEG: TrackDirection = TrackDirection(false);

    pub fn is_pos(&self) -> bool {
        self.0
    }

    pub fn inverse(&self) -> Self {
        Self(!self.0)
    }

    pub fn signum(&self) -> f32 {
        if self.is_pos() {
            1.0
        } else {
            -1.0
        }
    }
}

impl From<f32> for TrackDirection {
    fn from(f: f32) -> Self {
        Self(f > 0.)
    }
}

impl From<TrackDirection> for f32 {
    fn from(dir: TrackDirection) -> Self {
        if dir.0 {
            1.0
        } else {
            -1.0
        }
    }
}

impl Mul<f32> for TrackDirection {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        f32::from(self) * rhs
    }
}

impl Mul<TrackDirection> for TrackDirection {
    type Output = TrackDirection;

    fn mul(self, rhs: TrackDirection) -> Self::Output {
        Self(!(self.0 ^ rhs.0))
    }
}

pub struct TrackData {
    pub segment: TrackSegment,
    pub curve: CubicBezierSegment<f32>,
    pub length: f32,
}

impl TrackData {
    pub fn start_tile(&self) -> TileIndex {
        self.segment.start.tile
    }

    pub fn end_tile(&self) -> TileIndex {
        self.segment.end.tile
    }

    pub fn get_pos(&self, direction: TrackDirection) -> TrackPos {
        if direction.is_pos() {
            self.segment.end
        } else {
            self.segment.start
        }
    }
}

impl From<TrackSegment> for TrackData {
    fn from(segment: TrackSegment) -> Self {
        let (start, ctrl1, ctrl2, end) = segment.control_points();
        let curve = CubicBezierSegment {
            from: Point::new(start.x, start.y),
            ctrl1: Point::new(ctrl1.x, ctrl1.y),
            ctrl2: Point::new(ctrl2.x, ctrl2.y),
            to: Point::new(end.x, end.y),
        };
        Self {
            segment,
            curve,
            length: curve.approximate_length(0.1),
        }
    }
}

#[derive(Default)]
pub struct Network {
    pathing_graph: DiGraphMap<TrackPos, TrackEdge>,
    pub tracks: HashMap<TrackID, TrackData>,
}

impl Network {
    pub fn get_connections(&self, tile: TileIndex) -> [bool; 8] {
        let mut exists = [false; 8];
        for (i, exists) in exists.iter_mut().enumerate() {
            let node = TrackPos {
                facing: i.into(),
                tile,
            };
            *exists = self.pathing_graph.contains_node(node);
        }
        exists
    }

    pub fn add_track(&mut self, segment: TrackSegment) -> TrackID {
        let id = NEXT_TRACK_ID.fetch_add(1, Ordering::SeqCst);
        self.pathing_graph
            .add_edge(segment.start, segment.end.inverse(), TrackEdge::pos(id));
        self.pathing_graph
            .add_edge(segment.end, segment.start.inverse(), TrackEdge::neg(id));

        self.tracks.insert(id, TrackData::from(segment));

        id
    }

    pub fn get(&self, id: TrackID) -> Option<&TrackData> {
        self.tracks.get(&id)
    }

    pub fn get_data(&self, edge: TrackEdge) -> Option<&TrackData> {
        self.tracks.get(&edge.track)
    }

    pub fn remove_track(&mut self, id: TrackID) {
        let track = self.tracks.remove(&id);
        if let Some(track) = track {
            let segment = track.segment;
            self.pathing_graph
                .remove_edge(segment.start, segment.end.inverse());
            self.pathing_graph
                .remove_edge(segment.end, segment.start.inverse());
        }
    }

    pub fn get_exits(&self, node: &TrackPos) -> Vec<(&TrackEdge, &TrackData)> {
        let node = node.inverse();
        self.pathing_graph
            .edges(node)
            .map(|(_, _, edge)| (edge, self.get(edge.track).unwrap()))
            .collect()
    }
}

#[derive(Clone, Copy, Default, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct TrackEdge {
    pub track: TrackID,
    pub direction: TrackDirection,
}

impl TrackEdge {
    pub fn pos(id: TrackID) -> Self {
        Self {
            track: id,
            direction: TrackDirection::POS,
        }
    }

    pub fn neg(id: TrackID) -> Self {
        Self {
            track: id,
            direction: TrackDirection::NEG,
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
        network.add_track(*segment);
        render.send(NetworkRenderEvent);
    }
}

pub fn remove_tracks(
    mut network: ResMut<Network>,
    tracks: Query<(&Hover, &NetworkTrack)>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut render: EventWriter<NetworkRenderEvent>,
) {
    if mouse_buttons.pressed(MouseButton::Right) {
        tracks.for_each(|(h, track)| {
            if h.hovered() {
                network.remove_track(track.0);
                render.send(NetworkRenderEvent);
            }
        });
    }
}
