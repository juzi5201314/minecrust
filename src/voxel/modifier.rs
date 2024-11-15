use bevy::math::IVec3;
use bevy::prelude::Resource;

use super::voxel_block::BlockId;

#[derive(Resource)]
pub struct VoxelModifier {
    pub queue: (
        kanal::Sender<(IVec3, BlockId)>,
        kanal::Receiver<(IVec3, BlockId)>,
    ),
    //pub queue: Vec<(IVec3, VoxelBlock)>,
}

impl VoxelModifier {
    pub fn set(&self, position: IVec3, block: BlockId) {
        self.queue.0.send((position, block));
    }
}

impl Default for VoxelModifier {
    fn default() -> Self {
        Self {
            queue: kanal::unbounded(),
        }
    }
}
