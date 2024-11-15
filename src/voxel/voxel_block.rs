use block_mesh::{MergeVoxel, Voxel};
use once_cell::sync::Lazy;

use crate::atom::Atom;

pub static AIR: Lazy<BlockId> = Lazy::new(|| Atom::new("core::air"));

pub type BlockId = Atom;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum VoxelBlock {
    #[default]
    Air,
    Solid(u16),
}

impl VoxelBlock {
    #[inline]
    pub fn is_solid(&self) -> bool {
        !self.is_air()
    }

    #[inline]
    pub fn is_air(&self) -> bool {
        matches!(self, VoxelBlock::Air)
    }

    /* pub fn from_id(id: &str) -> Self {
        match id {
            "core::air" => VoxelBlock::Air,
            _ => VoxelBlock::Solid(Atom::new(id)),
        }
    } */
}

impl Voxel for VoxelBlock {
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        match self {
            VoxelBlock::Air => block_mesh::VoxelVisibility::Empty,
            VoxelBlock::Solid(_) => block_mesh::VoxelVisibility::Opaque,
        }
    }
}

impl MergeVoxel for VoxelBlock {
    type MergeValue = u16;

    fn merge_value(&self) -> Self::MergeValue {
        match self {
            VoxelBlock::Air => 0,
            VoxelBlock::Solid(id) => *id,
        }
    }
}

impl bincode::Encode for VoxelBlock {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        (match self {
            VoxelBlock::Air => 0,
            VoxelBlock::Solid(idx) => *idx,
        })
        .encode(encoder)
    }
}

impl bincode::Decode for VoxelBlock {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let idx = u16::decode(decoder)?;
        Ok(if idx == 0 {
            VoxelBlock::Air
        } else {
            VoxelBlock::Solid(idx)
        })
    }
}

bincode::impl_borrow_decode!(VoxelBlock);
