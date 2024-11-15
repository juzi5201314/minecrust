use std::sync::Arc;

use ahash::AHashMap;
use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec3, Vec3A};
use bevy::prelude::{Component, Entity, Resource};

use super::chunk::ChunkData;
use super::generator::flat::FlatGenerator;
use super::generator::Generator;

// All chunks in the world are children of root
#[derive(Component)]
pub struct WorldRoot;

#[derive(Resource)]
pub struct VoxelWorld {
    pub loaded_chunks: scc::HashMap<IVec3, ChunkData, ahash::RandomState>,
    pub loading_chunks: scc::HashSet<IVec3, ahash::RandomState>,
    pub saving_chunks: Arc<scc::HashSet<IVec3, ahash::RandomState>>,
    pub generator: Arc<dyn Generator>,
    pub bounds: Aabb3d,
    pub root: Entity,
}

impl VoxelWorld {
    pub fn with_generator(mut self, generator: impl Generator + 'static) -> Self {
        self.generator = Arc::new(generator);
        self
    }

    pub fn can_load(&self, pos: IVec3) -> bool {
        !(self.loaded_chunks.contains(&pos)
            || self.loading_chunks.contains(&pos)
            || self.saving_chunks.contains(&pos))
    }

    /* pub fn get_chunk_ref(&self, pos: IVec3) -> ChunkRef {
        let mut chunks = ChunkRef {
            refs: Vec::with_capacity(27),
        };
        for i in 0..27 {
            let offset = index_to_ivec3_bounds(i, 3) + IVec3::splat(-1);

            chunks
                .refs
                .push(Arc::clone(self.chunks.get(&(pos + offset)).unwrap()));
        }
        chunks
    } */
}

impl Default for VoxelWorld {
    fn default() -> Self {
        Self {
            loaded_chunks: Default::default(),
            loading_chunks: Default::default(),
            saving_chunks: Default::default(),
            bounds: Aabb3d::new(Vec3A::ZERO, Vec3A::ZERO),
            root: Entity::PLACEHOLDER,
            generator: Arc::new(FlatGenerator::new()),
        }
    }
}
