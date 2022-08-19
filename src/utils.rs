use std::{f32::consts::PI, ops::Add};

use super::*;

pub type TileIndex = (i32, i32);

#[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Octant(pub i8);

#[allow(dead_code)]
impl Octant {
    pub fn left(&self) -> Self {
        Self((self.0 - 1) % 8)
    }

    pub fn right(&self) -> Self {
        Self((self.0 + 1) % 8)
    }

    pub fn inverse(&self) -> Self {
        Self((self.0 + 4) % 8)
    }

    pub fn perp(&self) -> Self {
        Self((self.0 + 2) % 8)
    }
}

impl Add for Octant {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self((self.0 + rhs.0) % 8)
    }
}

impl From<usize> for Octant {
    fn from(o: usize) -> Self {
        Self((o % 8) as i8)
    }
}

impl From<f32> for Octant {
    fn from(o: f32) -> Self {
        Self((o % 8.) as i8)
    }
}

pub fn octant_to_unit<T>(octant: T) -> Vec2
where
    T: Into<Octant>,
{
    let octant = octant.into();
    let angle = octant_to_angle(octant);
    angle_to_unit(angle)
}

pub fn octant_to_angle<T>(octant: T) -> f32
where
    T: Into<Octant>,
{
    octant.into().0 as f32 * PI / 4.
}

pub fn angle_to_unit(angle: f32) -> Vec2 {
    Vec2::new(f32::sin(angle), f32::cos(angle))
}

pub type TileVec = IVec2;

// Convert from pos (Vec2)
pub fn pos_to_vec(pos: Vec2) -> Vec2 {
    (pos / TILE_SIZE - 0.5).round()
}

pub fn pos_to_tile_vec(pos: Vec2) -> TileVec {
    pos_to_vec(pos).as_ivec2()
}

pub fn pos_to_tile(pos: Vec2) -> TileIndex {
    let vec = pos_to_tile_vec(pos);
    (vec.x, vec.y)
}

// Convert from tile vectors (IVec2)
pub fn tile_vec_to_pos(tile: TileVec) -> Vec2 {
    tile.as_vec2() * TILE_SIZE
}

pub fn tile_vec_to_center(tile: TileVec) -> Vec2 {
    tile_vec_to_pos(tile) + Vec2::splat(TILE_SIZE / 2.)
}

//Convert from tile index (i32, i32)
pub fn tile_to_vec(tile: TileIndex) -> TileVec {
    TileVec::new(tile.0, tile.1)
}

pub fn tile_to_center(tile: TileIndex) -> Vec2 {
    tile_vec_to_center(tile_to_vec(tile))
}
