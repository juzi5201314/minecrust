use std::sync::Arc;

use serde::Deserialize;

use crate::atom::Atom;
use crate::voxel::textures::Face;

#[derive(Debug, Clone, Deserialize)]
pub struct BlockRegistry {
    #[serde(alias = "name")]
    pub id: Atom,
    pub metadata: Arc<BlockMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockMetadata {
    pub textures: BlockTextures,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockTextures {
    pub top: String,
    pub bottom: Option<String>,
    pub side: Option<String>,
    pub front: Option<String>,
    pub back: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

impl BlockTextures {
    #[inline]
    pub fn top(&self) -> &str {
        &self.top
    }

    #[inline]
    pub fn bottom(&self) -> &str {
        self.bottom
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or_else(|| self.top())
    }

    #[inline]
    pub fn side(&self) -> &str {
        self.side
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or_else(|| self.top())
    }

    #[inline]
    pub fn front(&self) -> &str {
        self.front
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or_else(|| self.side())
    }

    #[inline]
    pub fn back(&self) -> &str {
        self.back
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or_else(|| self.side())
    }

    #[inline]
    pub fn left(&self) -> &str {
        self.left
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or_else(|| self.side())
    }

    #[inline]
    pub fn right(&self) -> &str {
        self.right
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or_else(|| self.side())
    }

    #[inline]
    pub fn face(&self, face: Face) -> &str {
        match face {
            Face::Top => self.top(),
            Face::Bottom => self.bottom(),
            Face::Front => self.front(),
            Face::Back => self.back(),
            Face::Left => self.left(),
            Face::Right => self.right(),
        }
    }
}
