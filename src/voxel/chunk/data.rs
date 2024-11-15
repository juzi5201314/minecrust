use bevy::math::IVec3;
use bevy::prelude::Entity;
use bincode::Encode;

use crate::voxel::palette::Palette;
use crate::voxel::voxel_block::VoxelBlock;

use super::ChunkData;

#[derive(Debug, PartialEq)]
pub struct Inner<'a> {
    pub pos: IVec3,
    pub palette: &'a Palette,
    pub voxels: &'a [VoxelBlock],
}

impl<'a> Encode for Inner<'a> {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.pos.to_array().encode(encoder)?;
        self.palette.encode(encoder)?;
        self.voxels.encode(encoder)?;
        Ok(())
    }
}

impl<'a> bincode::BorrowDecode<'a> for ChunkData {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'a>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        // order == `Inner::encode`
        let pos = IVec3::from_array(<_>::borrow_decode(decoder)?);
        let palette = <_>::borrow_decode(decoder)?;
        let voxels = <_>::borrow_decode(decoder)?;

        Ok(ChunkData {
            pos,
            voxels,
            solid_count: 0,
            uniform: false,
            hash: 0,
            entity: Entity::PLACEHOLDER,
            palette,
        })
    }
}
