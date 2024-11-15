use minecrust::core::CorePlugin;

use bevy::diagnostic::*;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};
use minecrust::loading::loading_complete;
use minecrust::perf_ui::PerfUiPlugin;
use minecrust::script::ScriptPlugin;
use minecrust::state::AppState;
use minecrust::voxel::VoxelEnginePlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: bevy::window::PresentMode::AutoNoVsync,
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::VULKAN),
                        dx12_shader_compiler: bevy::render::settings::Dx12Compiler::Dxc {
                            dxil_path: None,
                            dxc_path: None,
                        },
                        ..Default::default()
                    }),
                    synchronous_pipeline_compilation: false,
                })
                .set(ImagePlugin::default_nearest()),
        )
        // states
        .init_state::<AppState>()
        .add_loading_state(
            LoadingState::new(AppState::PrepareAssets).continue_to_state(AppState::Loading),
        )
        .add_plugins(WireframePlugin)
        .add_plugins((
            FrameTimeDiagnosticsPlugin,
            //LogDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
        ))
        .add_plugins((VoxelEnginePlugin, CorePlugin, PerfUiPlugin, ScriptPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, loading_complete.run_if(in_state(AppState::Loading)))
        .insert_resource(Msaa::Off)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wireframe_config: ResMut<WireframeConfig>,
) {
    commands.spawn(Camera3dBundle {
        camera_3d: Camera3d {
            screen_space_specular_transmission_steps: 0,
            ..Default::default()
        },
        camera: Camera {
            order: -1,
            is_active: false,
            ..Default::default()
        },
        ..Default::default()
    });
    //wireframe_config.global = true;
}
