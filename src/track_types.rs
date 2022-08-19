use super::*;

#[derive(Debug, Clone, Copy, Default, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct TrackPos {
    pub tile: TileIndex,
    pub facing: Octant,
}

impl TrackPos {
    pub fn new(tile: TileIndex, facing: Octant) -> Self {
        Self { tile, facing }
    }

    pub fn inverse(&self) -> Self {
        Self {
            tile: self.tile,
            facing: self.facing.inverse(),
        }
    }
}

impl From<(IVec2, Octant)> for TrackPos {
    fn from((vec, facing): (IVec2, Octant)) -> Self {
        Self {
            tile: (vec.x, vec.y),
            facing,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TrackSegment {
    pub start: TrackPos,
    pub end: TrackPos,
}

impl TrackSegment {
    pub fn from_directed(start: TrackPos, end: TrackPos) -> Self {
        Self {
            start,
            end: end.inverse(),
        }
        .sort()
    }

    fn sort(mut self) -> Self {
        if self.start.tile > self.end.tile {
            std::mem::swap(&mut self.start, &mut self.end);
        }
        self
    }

    pub fn control_points(&self) -> (Vec2, Vec2, Vec2, Vec2) {
        let start_pos = tile_to_center(self.start.tile);
        let end_pos = tile_to_center(self.end.tile);

        let ctrl_mag = start_pos.distance(end_pos) / 3.;
        let start_ctrl = octant_to_unit(self.start.facing);
        let end_ctrl = octant_to_unit(self.end.facing);
        (
            start_pos,
            start_pos + start_ctrl * ctrl_mag,
            end_pos + end_ctrl * ctrl_mag,
            end_pos,
        )
    }
}

impl From<(TileIndex, TileIndex, &TrackEdge)> for TrackSegment {
    fn from((start, end, edge): (TileIndex, TileIndex, &TrackEdge)) -> Self {
        if start < end {
            Self {
                start: TrackPos::new(start, edge.start),
                end: TrackPos::new(end, edge.end),
            }
        } else {
            Self {
                start: TrackPos::new(end, edge.start),
                end: TrackPos::new(start, edge.end),
            }
        }
    }
}
