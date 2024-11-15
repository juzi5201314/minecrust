use std::collections::VecDeque;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::Arc;

use ahash::{AHashMap, HashSet};
use anyhow::Context;
use bevy::math::bounding::Aabb3d;
use bevy::math::{UVec3, Vec3A};
use bevy::prelude::*;
use bevy::tasks::futures_lite::future::poll_once;
use bevy::tasks::{block_on, AsyncComputeTaskPool};
use ndshape::{ConstShape, ConstShape3u32};
use parking_lot::RwLock;
use redb::ReadableTable;

use crate::core::registry::Registry;
use crate::voxel::chunk_task::GenMeshTaskData;

use super::chunk_task::{BuildChunkTask, BuildChunkTaskInner, GenMeshTask};
use super::config::VoxelConfig;
use super::material::VoxelMaterialHandle;
use super::mesh::{MeshCache, MeshRef};
use super::modifier::VoxelModifier;
use super::palette::Palette;
use super::storage::{WorldDatabase, CHUNKS};
use super::textures::TextureMap;
use super::voxel_block::{BlockId, VoxelBlock};
use super::world::{VoxelWorld, WorldRoot};
use super::VoxelWorldCamera;

mod data;

pub const CHUNK_SIZE: u32 = 32;
// with 1-voxel boundary padding. but....why?
const PADDED_CHUNK_SIZE: u32 = CHUNK_SIZE + 2;
pub type PaddedChunkShape = ConstShape3u32<PADDED_CHUNK_SIZE, PADDED_CHUNK_SIZE, PADDED_CHUNK_SIZE>;

//pub type VoxelArray = [VoxelBlock; PaddedChunkShape::SIZE as usize];
pub type VoxelArray = Vec<VoxelBlock>;

/* #[derive(Resource, Deref, DerefMut, Default)]
pub struct ChunkLoadBuffer(#[deref] Vec<(IVec3, ChunkData)>); */

#[derive(Resource, Deref, DerefMut)]
pub struct ChunkLoadQueue(#[deref] (kanal::Sender<IVec3>, kanal::Receiver<IVec3>));

impl Default for ChunkLoadQueue {
    fn default() -> Self {
        ChunkLoadQueue(kanal::unbounded())
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct ChunkUpdateBuffer(#[deref] Vec<(IVec3, ChunkData)>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct ChunkUnloadBuffer(#[deref] Vec<IVec3>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct MeshCacheBuffer(#[deref] Vec<(u64, MeshRef)>);

// modified but in unloaded chunk
#[derive(Resource, Clone, Deref, DerefMut, Default)]
pub struct ModifiedVoxels(#[deref] Arc<RwLock<AHashMap<IVec3, BlockId>>>);

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct NeedRemesh;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct NeedUnload;

#[derive(Component)]
pub struct Modified();

#[derive(Component)]
pub struct Chunk {
    pub position: IVec3,
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct ChunkData {
    pub pos: IVec3,
    pub voxels: VoxelArray,
    pub solid_count: u32,
    pub uniform: bool,
    pub hash: u64,
    pub entity: Entity,
    pub palette: Palette,
}

impl ChunkData {
    pub fn new(pos: IVec3, entity: Entity) -> Self {
        ChunkData {
            pos,
            entity,
            //voxels: [const { VoxelBlock::Air }; PaddedChunkShape::SIZE as usize],
            voxels: vec![VoxelBlock::Air; PaddedChunkShape::SIZE as usize],
            solid_count: 0,
            uniform: false,
            hash: 0,
            palette: Palette::default(),
        }
    }

    pub fn empty() -> Self {
        ChunkData::new(IVec3::ZERO, Entity::PLACEHOLDER)
    }

    pub fn generate_hash(&mut self) {
        let mut hasher = ahash::RandomState::new().build_hasher();
        for block in &self.voxels {
            block.hash(&mut hasher);
        }
        self.hash = hasher.finish();
    }

    #[inline]
    /// `None` == Air
    pub fn get_block_id(&self, index: u32) -> Option<&BlockId> {
        let idx = match &self.voxels[index as usize] {
            VoxelBlock::Air => return None,
            VoxelBlock::Solid(idx) => *idx,
        };
        self.palette.block_id(idx)
    }

    /* #[inline]
    pub fn get_block_with_pos(&self, pos: UVec3) -> &VoxelBlock {
        self.get_block(PaddedChunkShape::linearize(pos.to_array()))
    } */

    pub fn set_block(&mut self, pos: UVec3, block_id: &BlockId) {
        let voxel = self.palette.voxel_block(&block_id);
        self.voxels[PaddedChunkShape::linearize(pos.to_array()) as usize] = voxel;
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.solid_count == PaddedChunkShape::SIZE
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.solid_count == 0
    }

    /// `ChunkData` -> `bincode::encode` -> `zstd::encode` -> bytes
    /// write 8 bytes hash to header.
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let inner = data::Inner {
            pos: self.pos,
            palette: &self.palette,
            voxels: &self.voxels,
        };
        let mut buffer = Vec::with_capacity(65536);
        let mut encoder = zstd::Encoder::new(&mut buffer, 0).with_context(|| "zstd")?;
        
        bincode::encode_into_std_write(inner, &mut encoder, bincode::config::standard())
            .with_context(|| "bincode")?;
        encoder.finish()?;
        buffer.splice(0..0, self.hash.to_le_bytes());
        Ok(buffer)
    }

    /// bytes -> `zstd::decode` -> `bincode::decode` -> `ChunkData`
    pub fn decode(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut buffer = Vec::with_capacity(bytes.len() * 2);
        // skip first 8 bytes as hash
        let mut decoder = zstd::Decoder::new(&bytes[8..])?;
        std::io::copy(&mut decoder, &mut buffer)?;

        let (data, _) = bincode::borrow_decode_from_slice(&buffer, bincode::config::standard())?;

        Ok(data)
    }

    #[inline]
    pub fn read_header(bytes: &[u8]) -> anyhow::Result<u64> {
        Ok(u64::from_le_bytes(
            bytes[..8]
                .try_into()
                .with_context(|| "read chunk hash but data length < 8")?,
        ))
    }
}

/* impl Clone for ChunkData {
    fn clone(&self) -> Self {
        Self {
            position: self.position,
            voxels: self
                .voxels
                .iter()
                .map(|v| ArcSwap::new(v.load_full()))
                .collect(),
            solid_count: self.solid_count,
            uniform: self.uniform,
            hash: self.hash,
            entity: self.entity,
            palette: self.palette.clone(),
        }
    }
} */

pub fn load_visit_chunks(
    mut commands: Commands,
    //mut chunk_load_buffer: ResMut<ChunkLoadBuffer>,
    load_queue: ResMut<ChunkLoadQueue>,
    config: Res<VoxelConfig>,
    world: Res<VoxelWorld>,
    camera: Query<(&Camera, &GlobalTransform), With<VoxelWorldCamera>>,
) {
    let (camera, cam_gtf) = camera.single();
    let cam_pos = cam_gtf.translation().as_ivec3();

    let spawning_distance = config.spawning_distance;

    let viewport_size = camera.physical_viewport_size().unwrap_or_default();

    let mut visited = HashSet::default();
    let mut chunks_deque =
        VecDeque::with_capacity((config.spawning_rays * spawning_distance) as usize);

    // Shoots a ray from the given point, and queue all (non-spawned) chunks intersecting the ray
    for _ in 0..config.spawning_rays {
        let m = config.spawning_ray_margin;
        let random_point_in_viewport = {
            let x = rand::random::<f32>() * (viewport_size.x + m * 2) as f32 - m as f32;
            let y = rand::random::<f32>() * (viewport_size.y + m * 2) as f32 - m as f32;
            Vec2::new(x, y)
        };

        let Some(ray) = camera.viewport_to_world(cam_gtf, random_point_in_viewport) else {
            return;
        };
        let mut current = ray.origin;
        let mut t = 0f32;

        while t < (spawning_distance * CHUNK_SIZE) as f32 {
            let chunk_pos = current.as_ivec3() / CHUNK_SIZE as i32;
            if let Some(chunk) = world.loaded_chunks.get(&chunk_pos) {
                if chunk.is_full() {
                    // If we hit a full chunk, we can stop the ray early
                    break;
                }
            } else {
                chunks_deque.push_back(chunk_pos);
            }
            t += CHUNK_SIZE as f32;
            current = ray.origin + ray.direction * t;
        }

        // We also queue the chunks closest to the camera to make sure they will always spawn early
        let chunk_at_camera = cam_pos / CHUNK_SIZE as i32;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let queue_pos = chunk_at_camera + IVec3::new(x, y, z);
                    chunks_deque.push_back(queue_pos);
                }
            }
        }

        // Then, when we have a queue of chunks, we can set them up for spawning
        while let Some(chunk_position) = chunks_deque.pop_front() {
            if visited.contains(&chunk_position)
                || chunks_deque.len() > config.max_spawn_per_frame as usize
            {
                continue;
            }
            visited.insert(chunk_position);

            if chunk_position.distance_squared(chunk_at_camera) > (spawning_distance ^ 2) as i32 - 1
            {
                continue;
            }

            if world.can_load(chunk_position) {
                (&**load_queue).0.send(chunk_position).unwrap();
            }

            /* if !has_chunk {
                let chunk_entity = commands.spawn(NeedRemesh).id();
                commands
                    .entity(root.get_single().unwrap())
                    .add_child(chunk_entity);
                let chunk = Chunk {
                    position: chunk_position,
                    entity: chunk_entity,
                };

                chunk_load_buffer.push((
                    chunk_position,
                    ChunkData {
                        entity: chunk.entity,
                        position: chunk_position,
                        ..ChunkData::new()
                    },
                ));

                commands.entity(chunk.entity).try_insert((
                    chunk,
                    Transform::from_translation(
                        chunk_position.as_vec3() * CHUNK_SIZE as f32 - 1f32,
                    ),
                ));
            } else {
                continue;
            } */

            /* if configuration.chunk_spawn_strategy() != ChunkSpawnStrategy::Close {
                continue;
            } */

            // If we get here, we queue the neighbors
            /* for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let queue_pos = chunk_position + IVec3::new(x, y, z);
                        if queue_pos == chunk_position {
                            continue;
                        }
                        chunks_deque.push_back(queue_pos);
                    }
                }
            } */
        }
    }
}

pub fn mark_unload_chunks(
    mut commands: Commands,
    mut unload_buffer: ResMut<ChunkUnloadBuffer>,
    config: Res<VoxelConfig>,
    world: Res<VoxelWorld>,
    camera_gtf: Query<&GlobalTransform, With<VoxelWorldCamera>>,
    chunks: Query<&Chunk, (Without<GenMeshTask>, Without<NeedRemesh>)>,
) {
    let camera_gtf = camera_gtf.single();
    let cam_pos = camera_gtf.translation().as_ivec3();
    let cam_at_chunk = cam_pos / CHUNK_SIZE as i32;

    for chunk in chunks.iter() {
        if chunk.position.distance_squared(cam_at_chunk) > (config.spawning_distance as i32 ^ 2) - 1
        {
            // remove it
            // commands.entity(chunk.entity).try_insert(NeedUnload);
            if world.loaded_chunks.contains(&chunk.position) {
                commands.entity(chunk.entity).despawn_recursive();
                unload_buffer.push(chunk.position);
            }
        }
    }
}

pub fn unload_chunks(
    mut world: ResMut<VoxelWorld>,
    mut chunk_unload_buffer: ResMut<ChunkUnloadBuffer>,
    storage: Res<WorldDatabase>,
) {
    if chunk_unload_buffer.is_empty() {
        return;
    }

    // load buffer (write) -> world.chunks
    /* for (pos, data) in chunk_load_buffer.drain(..) {
        world.chunks.insert(pos, data);
        let position_f = Vec3A::from(pos.as_vec3());
        if position_f.cmplt(world.bounds.min).any() {
            world.bounds.min = position_f.min(world.bounds.min);
        } else if position_f.cmpgt(world.bounds.max).any() {
            world.bounds.max = position_f.max(world.bounds.max);
        }
    } */

    /* for (pos, mut data) in chunk_update_buffer.drain(..) {
        data.pos = pos;
        world.loaded_chunks.insert(pos, data);
        let position_f = Vec3A::from(pos.as_vec3());
        if position_f.cmplt(world.bounds.min).any() {
            world.bounds.min = position_f.min(world.bounds.min);
        } else if position_f.cmpgt(world.bounds.max).any() {
            world.bounds.max = position_f.max(world.bounds.max);
        }
    } */

    let pool = AsyncComputeTaskPool::get();
    let mut need_rebuild_aabb = false;
    for pos in chunk_unload_buffer.drain(..) {
        let (_, chunk) = world
            .loaded_chunks
            .remove(&pos)
            .expect("remove chunk but it not in the world");
        world.saving_chunks.insert(pos).ok();

        let storage = storage.clone();
        let saving_chunk = world.saving_chunks.clone();
        pool.spawn(async move {
            let _span = tracing::info_span!("profiling::{save chunk}").entered();
            storage
                .write(CHUNKS, |_, mut table| {
                    let key = chunk.pos.to_array();
                    // if hash of the chunk to be saved == existing chunk,
                    // then skip. (This means that the chunk has not changed)
                    if let Some(data) = table.get(&key)? {
                        let hash = ChunkData::read_header(data.value())?;
                        if hash == chunk.hash {
                            return Ok(());
                        }
                    }
                    let data = chunk.encode().with_context(|| "encode chunk data failed")?;
                    table.insert(key, &*data)?;
                    Ok(())
                })
                .and_then(|v| v)
                .expect("save chunk data failed");
            saving_chunk.remove(&chunk.pos);
        })
        .detach();

        // TODO: rebuild aabb
        need_rebuild_aabb = world.bounds.min.floor().as_ivec3() == pos
            || world.bounds.max.floor().as_ivec3() == pos;
    }

    if need_rebuild_aabb {
        let _span = tracing::info_span!("profiling::{rebuild aabb}").entered();
        let mut tmp_vec = Vec::with_capacity(world.loaded_chunks.len());
        world.loaded_chunks.scan(|k, _| {
            tmp_vec.push(Vec3A::from(k.as_vec3()));
        });
        world.bounds = Aabb3d::from_point_cloud(Vec3A::ZERO, Quat::IDENTITY, tmp_vec.drain(..));
    }
}

pub fn flush_mesh_cache(
    mut mesh_cache_buffer: ResMut<MeshCacheBuffer>,
    mesh_cache: Res<MeshCache>,
) {
    if !mesh_cache_buffer.is_empty() {
        let Some(mut write_lock) = mesh_cache.map.try_write() else {
            return;
        };
        for (hash, mesh) in mesh_cache_buffer.drain(..) {
            write_lock.insert(hash, mesh.0.clone());
        }
        write_lock.remove_expired();
    }
}

pub fn flush_voxel_write_buffer(
    mut commands: Commands,
    world: Res<VoxelWorld>,
    modifier: Res<VoxelModifier>,
    load_queue: Res<ChunkLoadQueue>,
    modified: Res<ModifiedVoxels>,
) {
    if modifier.queue.1.is_empty() {
        return;
    }

    while let Some((block_pos, block_id)) = modifier.queue.1.try_recv().unwrap() {
        let (chunk_pos, block_pos_in_chunk) = get_chunk_voxel_position(block_pos);
        //modified.write().insert(block_pos, block);

        let exist = world
            .loaded_chunks
            .update(&chunk_pos, |_, data| {
                data.set_block(block_pos_in_chunk, &block_id);
                commands.entity(data.entity).try_insert(NeedRemesh);
            })
            .is_some();
        if !exist {
            // if chunk not loaded, queue it for loading
            load_queue.0 .0.send(chunk_pos).unwrap();
            // add to ModifiedVoxels, automatically applied when chunks are loaded
            modified.write().insert(block_pos, block_id);
        }
    }
}

pub fn remesh_dirty_chunks(
    mut commands: Commands,
    registry: Res<Registry>,
    texture_map: Res<TextureMap>,
    mesh_cache: Res<MeshCache>,
    world: Res<VoxelWorld>,
    dirty_chunks: Query<&Chunk, With<NeedRemesh>>,
) {
    if dirty_chunks.is_empty() {
        return;
    }

    let pool = AsyncComputeTaskPool::get();
    for chunk in dirty_chunks.iter() {
        let mut task_data = GenMeshTaskData {
            position: chunk.position,
            chunk_data: world
                .loaded_chunks
                .get(&chunk.position)
                .expect("remesh chunk but not loaded")
                .clone(),
            mesh: None,
        };
        debug_assert_eq!(chunk.entity, task_data.chunk_data.entity);
        let mesh_cache = mesh_cache.clone();
        let registry = registry.clone();
        let texture_map = texture_map.clone();
        let task = pool.spawn(async move {
            //task_data.generate();

            if task_data.chunk_data.is_empty() {
                return task_data;
            }

            let cache_hit = mesh_cache.get(task_data.chunk_data.hash).is_some();
            if !cache_hit {
                task_data.generate_mesh(registry, texture_map);
            }

            task_data
        });

        commands
            .entity(chunk.entity)
            .insert(GenMeshTask(task))
            .remove::<NeedRemesh>();
    }
}

pub fn spawn_mesh(
    mut commands: Commands,
    mut tasks: Query<(&mut Chunk, &mut GenMeshTask, &Transform), Without<NeedRemesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mesh_cache_buffer: ResMut<MeshCacheBuffer>,
    mut chunk_update_buffer: ResMut<ChunkUpdateBuffer>,
    mesh_cache: Res<MeshCache>,
    material: Res<VoxelMaterialHandle>,
) {
    let mut i = 0;
    for (chunk, mut task, transform) in &mut tasks {
        if let Some(task_data) = block_on(poll_once(&mut task.0)) {
            debug_assert_eq!(chunk.entity, task_data.chunk_data.entity);
            if !task_data.chunk_data.is_empty() {
                let mesh_ref = if let Some(mesh) = mesh_cache.get(task_data.chunk_data.hash) {
                    mesh.clone()
                } else {
                    if let Some(mesh) = task_data.mesh {
                        let _ref = MeshRef(Arc::new(mesh_assets.add(mesh)));
                        mesh_cache_buffer.push((task_data.chunk_data.hash, _ref.clone()));
                        _ref
                    } else {
                        commands.entity(chunk.entity).try_insert(NeedRemesh);
                        continue;
                    }
                };

                commands
                    .entity(chunk.entity)
                    .try_insert(MaterialMeshBundle {
                        mesh: (&**mesh_ref).clone(),
                        material: material.clone(),
                        transform: *transform,
                        ..Default::default()
                    })
                    .try_insert(mesh_ref)
                    .remove::<bevy::render::primitives::Aabb>();

                //chunk_update_buffer.push((chunk.position, task_data.chunk_data));
            } else {
                commands
                    .entity(chunk.entity)
                    .remove::<Handle<Mesh>>()
                    .remove::<MeshRef>();
            }
            commands.entity(chunk.entity).remove::<GenMeshTask>();
            i += 1;
        }
    }
}

/// Returns a tuple of the chunk position and the voxel position within the chunk.
/// (block pos in world) -> (chunk pos, block pos in chunk)
#[inline]
pub fn get_chunk_voxel_position(position: IVec3) -> (IVec3, UVec3) {
    let chunk_position = IVec3 {
        x: (position.x as f32 / CHUNK_SIZE as f32).floor() as i32,
        y: (position.y as f32 / CHUNK_SIZE as f32).floor() as i32,
        z: (position.z as f32 / CHUNK_SIZE as f32).floor() as i32,
    };

    let voxel_position = (position - chunk_position * CHUNK_SIZE as i32).as_uvec3() + 1;

    (chunk_position, voxel_position)
}

/// ChunkLoadQueue -> spawn chunk entity & spawn GenChunkTask
pub fn load_chunks(
    mut commands: Commands,
    mut world: ResMut<VoxelWorld>,
    storage: Res<WorldDatabase>,
    load_queue: Res<ChunkLoadQueue>,
    modified: Res<ModifiedVoxels>,
) {
    if load_queue.1.is_empty() {
        return;
    }
    let pool = AsyncComputeTaskPool::get();

    while let Some(pos) = load_queue.1.try_recv().unwrap() {
        if !world.can_load(pos) {
            continue;
        }

        let chunk_entity = commands.spawn_empty().id();
        let chunk = Chunk {
            position: pos,
            entity: chunk_entity,
        };
        let generator = world.generator.clone();
        let modified = modified.clone();
        let storage = Some(storage.clone());
        let task = pool.spawn(async move {
            let inner = BuildChunkTaskInner {
                chunk_pos: pos,
                chunk_entity,
                modified_voxels: modified,
                generator,
                storage,
            };
            inner.build()
        });

        commands
            .entity(chunk.entity)
            .try_insert((
                chunk,
                Transform::from_translation(pos.as_vec3() * CHUNK_SIZE as f32 - 1f32),
            ))
            .insert(BuildChunkTask(task));
        // add chunk entity to world entity
        commands.entity(world.root).add_child(chunk_entity);

        world.loading_chunks.insert(pos).ok();

        let position_f = Vec3A::from(pos.as_vec3());
        if position_f.cmplt(world.bounds.min).any() {
            world.bounds.min = position_f.min(world.bounds.min);
        } else if position_f.cmpgt(world.bounds.max).any() {
            world.bounds.max = position_f.max(world.bounds.max);
        }
    }
}

/// GenChunkTask done -> add chunkdata to world.loaded_chunks
pub fn load_chunks_done(
    mut commands: Commands,
    mut tasks: Query<&mut BuildChunkTask>,
    world: Res<VoxelWorld>,
) {
    tasks
        .iter_mut()
        .filter_map(|mut task| block_on(poll_once(&mut task.0)))
        .for_each(|data| {
            let entity = data.entity;
            world.loading_chunks.remove(&data.pos);
            world.loaded_chunks.upsert(data.pos, data);
            commands
                .entity(entity)
                .insert(NeedRemesh)
                .remove::<BuildChunkTask>();
        });
}
