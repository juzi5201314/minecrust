use std::sync::Arc;

use ahash::AHashMap;
use bevy::prelude::{Deref, DerefMut, Resource};

pub type TexturesIndexMapper = Box<dyn Fn(u16) -> TextureBlock>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum Face {
    // Y+
    Top,
    // Y-
    Bottom,
    // X+
    Right,
    // X-
    Left,
    // Z+
    Front,
    // Z-
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureBlock {
    pub texture_index: u32,
    pub faces_offset: [u8; 6],
}

const _: () = debug_assert!(size_of::<TextureBlock>() == 12);

/// store idx(u28) and offset(u4) to one u32.
/// so, maximum texture index is 0xFFFFFFF0 (4294967280), offset is 0xF (16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextureIndex(u32);

impl TextureIndex {
    #[inline]
    pub fn idx(&self) -> u32 {
        self.0 >> 4
    }

    #[inline]
    pub fn offset(&self) -> u8 {
        (self.0 & 0xF) as u8
    }

    #[inline]
    pub fn new(idx: u32, offset: u8) -> Self {
        debug_assert!((idx <= 0xFFFFFFF0) && (offset <= 0xF));
        TextureIndex(idx << 4 | (offset as u32 & 0xF))
    }

    #[inline]
    pub fn set_idx(&mut self, idx: u32) {
        debug_assert!(idx <= 0xFFFFFFF0);
        self.0 = (idx << 4) | (self.0 & 0xF);
    }

    #[inline]
    pub fn set_offset(&mut self, offset: u8) {
        debug_assert!(offset <= 0xF);
        self.0 = (self.0 & 0xFFFFFFF0) | (offset as u32 & 0xF);
    }

    #[inline]
    pub fn zero() -> Self {
        TextureIndex(0)
    }

    #[inline]
    pub fn from_raw(raw: u32) -> Self {
        TextureIndex(raw)
    }

    #[inline]
    pub fn raw(&self) -> u32 {
        self.0
    }
}

impl TextureBlock {
    pub fn full(texture_index: u32) -> Self {
        TextureBlock {
            texture_index,
            faces_offset: [0; 6],
        }
    }

    pub fn with_top(mut self, offset: u8) -> Self {
        self.faces_offset[Face::Top as usize] = offset;
        self
    }

    pub fn with_bottom(mut self, offset: u8) -> Self {
        self.faces_offset[Face::Bottom as usize] = offset;
        self
    }

    pub fn with_right(mut self, offset: u8) -> Self {
        self.faces_offset[Face::Right as usize] = offset;
        self
    }

    pub fn with_left(mut self, offset: u8) -> Self {
        self.faces_offset[Face::Left as usize] = offset;
        self
    }

    pub fn with_front(mut self, offset: u8) -> Self {
        self.faces_offset[Face::Front as usize] = offset;
        self
    }

    pub fn with_back(mut self, offset: u8) -> Self {
        self.faces_offset[Face::Back as usize] = offset;
        self
    }

    pub fn with_side(self, offset: u8) -> Self {
        self.with_front(offset)
            .with_back(offset)
            .with_right(offset)
            .with_left(offset)
    }

    pub fn to_texture_index(&self, face: Face) -> TextureIndex {
        TextureIndex::new(self.texture_index, self.faces_offset[face as usize])
    }
}

#[derive(Resource, Deref, DerefMut, Clone)]
pub struct TextureMap(#[deref] pub Arc<AHashMap<String, usize>>);

#[test]
fn test_texture_index() {
    let mut idx = TextureIndex::new(114514, 15);
    assert_eq!((idx.idx(), idx.offset()), (114514, 15));
    idx.set_idx(10086);
    assert_eq!((idx.idx(), idx.offset()), (10086, 15));
    idx.set_offset(8);
    assert_eq!((idx.idx(), idx.offset()), (10086, 8));
}

pub fn default_texture_index_mapper() -> TexturesIndexMapper {
    Box::new(|ty| match ty {
        0 => TextureBlock::full(0),
        1 => TextureBlock::full(1),
        x => TextureBlock::full(x as u32),
    })
}
