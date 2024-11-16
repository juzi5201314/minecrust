use bevy::prelude::IVec3;

use crate::voxel::voxel_block::BlockId;

use super::Generator;

pub struct FlatGenerator;

impl Generator for FlatGenerator {
    fn generate(&self, pos: IVec3) -> BlockId {
        BlockId::new(if pos.y == 0 {
            "core::grass"
        } else {
            "core::air"
        })
    }
}

impl FlatGenerator {
    pub fn new() -> Self {
        FlatGenerator {}
    }
}
