use bevy::app::Plugin;
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy_asset_loader::loading_state::config::{ConfigureLoadingState, LoadingStateConfig};
use bevy_asset_loader::loading_state::LoadingStateAppExt;
use chunk::*;
use config::VoxelConfig;
use generator::noise::NoiseGenerator;
use material::{VoxelMaterial, VoxelMaterialHandle};
use mesh::MeshCache;
use modifier::VoxelModifier;
use textures_loader::{load_textures, unload_textures, BlockTextureAssets, VoxelTextures};
use world::{VoxelWorld, WorldRoot};

use crate::state::AppState;

pub mod chunk;
pub mod chunk_ref;
pub mod chunk_task;
pub mod config;
pub mod generator;
pub mod material;
pub mod mesh;
pub mod modifier;
pub mod palette;
pub mod storage;
pub mod textures;
pub mod textures_loader;
pub mod utils;
pub mod voxel_block;
pub mod world;

#[derive(Component)]
pub struct VoxelWorldCamera;

pub struct VoxelEnginePlugin;

impl Plugin for VoxelEnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelConfig>()
            .init_resource::<ModifiedVoxels>()
            .init_resource::<VoxelModifier>()
            .init_resource::<ChunkLoadQueue>()
            .init_resource::<ChunkUpdateBuffer>()
            .init_resource::<ChunkUnloadBuffer>()
            .init_resource::<MeshCacheBuffer>()
            .init_resource::<MeshCache>()
            .add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, VoxelMaterial>,
            >::default())
            .add_systems(OnEnter(AppState::InGame), (setup,))
            .add_systems(
                PreUpdate,
                (
                    (load_visit_chunks, load_chunks).chain(),
                    (remesh_dirty_chunks, load_chunks_done).chain(),
                    (flush_voxel_write_buffer, (flush_mesh_cache)).chain(),
                )
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(Update, (spawn_mesh,).run_if(in_state(AppState::InGame)))
            .add_systems(
                FixedUpdate,
                (mark_unload_chunks, unload_chunks).run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(AppState::Loading), load_textures)
            .add_systems(OnExit(AppState::Loading), unload_textures)
            .configure_loading_state(
                LoadingStateConfig::new(AppState::PrepareAssets)
                    .load_collection::<BlockTextureAssets>(),
            );
    }
}

fn setup(
    mut commands: Commands,
    mut material_assets: ResMut<Assets<ExtendedMaterial<StandardMaterial, VoxelMaterial>>>,
    voxel_texture: Res<VoxelTextures>,
) {
    let root = commands.spawn((
        WorldRoot,
        VisibilityBundle::default(),
        TransformBundle::default(),
    ));
    let mut world = VoxelWorld::default().with_generator(NoiseGenerator::new());

    world.root = root.id();
    commands.insert_resource(world);

    commands.insert_resource(VoxelMaterialHandle(material_assets.add(ExtendedMaterial {
        base: StandardMaterial {
            reflectance: 0.05,
            metallic: 0.05,
            perceptual_roughness: 0.95,
            ..Default::default()
        },
        extension: VoxelMaterial {
            array_texture: voxel_texture.0.clone(),
        },
    })));
}
