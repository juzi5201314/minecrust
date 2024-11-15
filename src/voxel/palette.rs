use indexmap::IndexSet;

use super::voxel_block::{BlockId, VoxelBlock, AIR};

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    map: IndexSet<BlockId, ahash::RandomState>,
}

impl Palette {
    fn mapped_idx(&mut self, name: &BlockId) -> u16 {
        if let Some(id) = self.map.get_index_of(name) {
            id as u16
        } else {
            let (id, _) = self.map.insert_full(name.clone());
            id as u16
        }
    }

    pub fn block_id(&self, idx: u16) -> Option<&BlockId> {
        self.map.get_index(idx as usize)
    }

    pub fn voxel_block(&mut self, id: &BlockId) -> VoxelBlock {
        if id == &*AIR {
            VoxelBlock::Air
        } else {
            VoxelBlock::Solid(self.mapped_idx(id))
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        let mut p = Palette {
            map: Default::default(),
        };
        p.mapped_idx(&AIR);
        p
    }
}

impl<'a> bincode::Encode for &'a Palette {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.map.len().encode(encoder)?;
        self.map
            .iter()
            .try_for_each(|atom| (&**atom).encode(encoder))
    }
}

impl<'a> bincode::BorrowDecode<'a> for Palette {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'a>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len = usize::borrow_decode(decoder)?;
        let mut map = IndexSet::with_capacity_and_hasher(len, ahash::RandomState::new());
        for _ in 0..len {
            let atom = <&str>::borrow_decode(decoder)?;
            assert!(
                map.insert(BlockId::new(atom)),
                "Duplicate block ID in palette"
            );
        }
        Ok(Palette { map })
    }
}
