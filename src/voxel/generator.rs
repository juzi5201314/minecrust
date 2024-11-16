use bevy::math::IVec3;

use super::voxel_block::BlockId;

pub mod flat;
pub mod noise;

pub trait Generator: Sync + Send {
    fn generate(&self, pos: IVec3) -> BlockId;
}