use std::sync::Arc;

use bevy::app::Plugin;
use bevy::pbr::ScreenSpaceAmbientOcclusionBundle;
use bevy::prelude::*;
use bevy::render::view::GpuCulling;
use bevy_asset_loader::loading_state::config::{ConfigureLoadingState, LoadingStateConfig};
use bevy_asset_loader::loading_state::LoadingStateAppExt;
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};
use registry::{register_core_items, Registry, RegistryAssets};

use crate::state::AppState;
use crate::voxel::modifier::VoxelModifier;
use crate::voxel::storage::WorldDatabase;
use crate::voxel::VoxelWorldCamera;

pub mod registry;
mod simple_control;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(NoCameraPlayerPlugin)
            .add_systems(OnEnter(AppState::Loading), register_core_items)
            .add_systems(
                OnEnter(AppState::InGame),
                ((setup_game,).chain(), setup_world_storage),
            )
            .init_resource::<Registry>()
            .configure_loading_state(
                LoadingStateConfig::new(AppState::PrepareAssets)
                    .load_collection::<RegistryAssets>(),
            );
    }
}

fn setup_game(mut commands: Commands, modifier: Res<VoxelModifier>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(50.0, 5.0, 50.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),

            ..Default::default()
        },
        VoxelWorldCamera,
        FlyCam,
        GpuCulling,
        //NoCpuCulling,
        ScreenSpaceAmbientOcclusionBundle::default(),
    ));
    commands.insert_resource(MovementSettings {
        sensitivity: 0.00006,
        speed: 24.0,
        ..Default::default()
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            ..Default::default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .looking_at(Vec3::new(-0.15, -0.1, 0.15), Vec3::Y),
        ..Default::default()
    });
    /* commands.insert_resource(AmbientLight {
        color: Color::srgb(0.98, 0.95, 0.82),
        brightness: 100.0,
    }); */

    for x in -10..10 {
        for z in -10..10 {
            //buffer.push((IVec3::new(x, 0, z), VoxelBlock::Solid(100)));
        }
    }
}

fn setup_world_storage(mut commands: Commands) {;
    commands.insert_resource(WorldDatabase::new("world").unwrap());
}
