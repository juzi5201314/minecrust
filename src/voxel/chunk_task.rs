use std::hash::{Hash, Hasher};
use std::sync::Arc;

use ahash::AHashSet;
use anyhow::Context;
use bevy::math::IVec3;
use bevy::prelude::{Component, Entity, Mesh};
use bevy::tasks::Task;
use ndshape::ConstShape as _;

use crate::core::registry::Registry;

use super::generator::Generator;
use super::mesh::generate_chunk_mesh;
use super::storage::{WorldDatabase, CHUNKS};
use super::textures::TextureMap;
use super::voxel_block::VoxelBlock;
use super::{ChunkData, ModifiedVoxels, PaddedChunkShape, CHUNK_SIZE};

#[derive(Component)]
pub struct GenMeshTask(pub Task<GenMeshTaskData>);

pub struct GenMeshTaskData {
    pub position: IVec3,
    pub chunk_data: ChunkData,
    pub mesh: Option<Mesh>,
}

impl GenMeshTaskData {
    pub fn generate_mesh(&mut self, registry: Registry, texture_map: TextureMap) {
        if self.mesh.is_none() && self.chunk_data.solid_count != 0 {
            self.mesh = Some(generate_chunk_mesh(
                &mut self.chunk_data,
                registry,
                texture_map,
            ));
        }
    }
}

#[derive(Component)]
pub struct BuildChunkTask(pub Task<ChunkData>);

pub struct BuildChunkTaskInner {
    pub chunk_pos: IVec3,
    pub chunk_entity: Entity,
    pub modified_voxels: ModifiedVoxels,
    pub generator: Arc<dyn Generator>,
    pub storage: Option<WorldDatabase>,
}

impl BuildChunkTaskInner {
    pub fn build(self) -> ChunkData {
        let mut filled_count = 0;
        let mut new_chunk = false;
        let mut hasher = ahash::AHasher::default();
        let mut modified_voxels = self.modified_voxels.write();
        let mut material_count = AHashSet::new();

        let mut chunk_data = if let Some(mut chunk_data) = self
            .storage
            .as_ref()
            .map(|storage| {
                let _span = tracing::info_span!("profiling::{read chunk from database}").entered();
                storage
                    .read(CHUNKS, |_, table| {
                        if let Some(bytes) = table.get(self.chunk_pos.to_array())? {
                            ChunkData::decode(bytes.value())
                                .map(Some)
                                .with_context(|| "decoding chunk data failed")
                        } else {
                            Ok(None)
                        }
                    })
                    .and_then(|v| v)
                    .expect("reading chunk data failed")
            })
            .flatten()
        {
            // read from database
            assert_eq!(self.chunk_pos, chunk_data.pos);

            //tracing::trace!("Load chunk:{} from database", self.chunk_pos);

            chunk_data.entity = self.chunk_entity;
            chunk_data
        } else {
            // generate new chunk from generator
            new_chunk = true;
            ChunkData::new(self.chunk_pos, self.chunk_entity)
        };

        let _span = new_chunk.then(|| tracing::info_span!("profiling::{generate block}").entered());

        // for each all blocks in the chunk
        for i in 0..PaddedChunkShape::SIZE {
            let pos = PaddedChunkShape::delinearize(i);
            let block_pos = IVec3 {
                x: pos[0] as i32 + (self.chunk_pos.x * CHUNK_SIZE as i32) - 1,
                y: pos[1] as i32 + (self.chunk_pos.y * CHUNK_SIZE as i32) - 1,
                z: pos[2] as i32 + (self.chunk_pos.z * CHUNK_SIZE as i32) - 1,
            };

            // apply modified voxels
            let modified = modified_voxels.remove(&block_pos);
            let voxel = if let Some(id) = modified {
                chunk_data.palette.voxel_block(&id)
            } else if new_chunk {
                chunk_data
                    .palette
                    .voxel_block(&self.generator.generate(block_pos))
            } else {
                chunk_data.voxels[i as usize]
            };

            voxel.hash(&mut hasher);

            if let VoxelBlock::Solid(id) = &voxel {
                material_count.insert(id.clone());
            }
            if !voxel.is_air() {
                filled_count += 1;
            }
            if new_chunk {
                chunk_data.voxels[i as usize] = voxel;
            }
        }

        chunk_data.solid_count = filled_count;
        chunk_data.hash = hasher.finish();
        if filled_count == 0 {
            // empty chunk, all is air
        } else if chunk_data.is_full() && material_count.len() == 1 {
            chunk_data.uniform = true;
        } else {
            // mixed
        }
        chunk_data
    }
}

pub struct SaveChunkTask(pub Task<()>);
