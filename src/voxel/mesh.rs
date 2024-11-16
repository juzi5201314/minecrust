use std::sync::{Arc, Weak};

use bevy::asset::Handle;
use bevy::prelude::{Component, Deref, Mesh, Resource};
use bevy::render::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::VertexFormat;
use block_mesh::{
    greedy_quads, GreedyQuadsBuffer, RIGHT_HANDED_Y_UP_CONFIG,
};
use ndshape::ConstShape;
use parking_lot::RwLock;
use weak_table::WeakValueHashMap;

use crate::core::registry::Registry;

use super::textures::{Face, TextureMap};
use super::voxel_block::VoxelBlock;
use super::{ChunkData, PaddedChunkShape, CHUNK_SIZE};

pub const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TextureIndex", 1034236490, VertexFormat::Uint32);

#[derive(Resource, Clone)]
pub struct MeshCache {
    pub map: Arc<RwLock<WeakValueHashMap<u64, Weak<Handle<Mesh>>, ahash::RandomState>>>,
}

#[derive(Component, Clone, Deref)]
pub struct MeshRef(#[deref] pub Arc<Handle<Mesh>>);

impl MeshCache {
    pub fn get(&self, voxel_hash: u64) -> Option<MeshRef> {
        self.map.read().get(&voxel_hash).map(MeshRef)
    }
}

impl Default for MeshCache {
    fn default() -> Self {
        Self {
            map: Arc::new(RwLock::new(WeakValueHashMap::default())),
        }
    }
}

pub fn generate_chunk_mesh(
    chunk_data: &mut ChunkData,
    registry: Registry,
    texture_map: TextureMap,
) -> Mesh {
    let _span = tracing::info_span!("profiling::{generate mesh}").entered();
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = GreedyQuadsBuffer::new(chunk_data.voxels.len());
    //let voxels = chunk_data.voxels.iter().collect::<Vec<_>>();
    greedy_quads(
        &chunk_data.voxels,
        &PaddedChunkShape {},
        [0; 3],
        [CHUNK_SIZE + 1; 3],
        &faces,
        &mut buffer,
    );
    let quads = buffer.quads;

    /* let mut quads = UnitQuadBuffer::new();
    visible_block_faces(
        &chunk_data.voxels,
        &PaddedChunkShape {},
        [0; 3],
        [CHUNK_SIZE + 1; 3],
        &faces,
        &mut quads,
    ); */

    let num_indices = quads.num_quads() * 6;
    let num_vertices = quads.num_quads() * 4;

    let mut indices = Vec::with_capacity(num_indices);
    let mut positions = Vec::with_capacity(num_vertices);
    let mut normals = Vec::with_capacity(num_vertices);
    let mut tex_coords = Vec::with_capacity(num_vertices);
    let mut texture_idxs = Vec::with_capacity(num_vertices);

    for (group, face) in quads.groups.into_iter().zip(faces.into_iter()) {
        for quad in group.into_iter() {
            indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));

            positions.extend_from_slice(&face.quad_mesh_positions(&quad.into(), 1.0));

            normals.extend_from_slice(&face.quad_mesh_normals());

            tex_coords.extend_from_slice(&face.tex_coords(
                RIGHT_HANDED_Y_UP_CONFIG.u_flip_face,
                true,
                &quad.into(),
            ));

            let normal = face.signed_normal();

            let voxel_index = PaddedChunkShape::linearize(quad.minimum) as usize;
            let voxel = &chunk_data.voxels[voxel_index];
            let block_idx = match voxel {
                VoxelBlock::Air => unreachable!("air block in mesh"),
                VoxelBlock::Solid(id) => id,
            };
            let block_id = chunk_data
                .palette
                .block_id(*block_idx)
                .expect("not found block in palette");

            let face = if normal.x > 0 {
                Face::Right
            } else if normal.x < 0 {
                Face::Left
            } else if normal.z > 0 {
                Face::Front
            } else if normal.z < 0 {
                Face::Back
            } else if normal.y > 0 {
                Face::Top
            } else if normal.y < 0 {
                Face::Bottom
            } else {
                unreachable!()
            };

            let idx = registry
                .get_block_with(block_id, |block| {
                    let path = block.metadata.textures.face(face);
                    *texture_map.get(path).expect("non-existent texture")
                })
                .unwrap();
            //let texture_idx = texture_index_mapper(block_type).to_texture_index(face);
            texture_idxs.extend_from_slice(&[idx as u32; 4]);
        }
    }

    let mut render_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    render_mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(positions),
    );
    render_mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(normals),
    );
    render_mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        VertexAttributeValues::Float32x2(tex_coords),
    );
    render_mesh.insert_attribute(
        ATTRIBUTE_TEXTURE_INDEX,
        VertexAttributeValues::Uint32(texture_idxs),
    );
    render_mesh.insert_indices(Indices::U32(indices.clone()));

    render_mesh
}
