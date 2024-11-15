use bevy::app::Plugin;
use bevy::prelude::*;
use iyes_perf_ui::entries::*;
use iyes_perf_ui::prelude::*;
use voxel::PerfUiEntryLoadedChunkCount;

use crate::state::AppState;

mod voxel;

pub struct PerfUiPlugin;

impl Plugin for PerfUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(iyes_perf_ui::PerfUiPlugin)
            .add_perf_ui_simple_entry::<PerfUiEntryLoadedChunkCount>()
            .add_systems(Startup, setup_perf_ui)
            .add_systems(OnEnter(AppState::InGame), add_perf_ui_entry::<PerfUiEntryLoadedChunkCount>)
            .add_systems(OnExit(AppState::InGame), remove_perf_ui_entry::<PerfUiEntryLoadedChunkCount>);
    }
}

fn setup_perf_ui(mut commands: Commands) {
    commands.spawn((
        PerfUiRoot::default(),
        PerfUiFramerateEntries::default(),
        PerfUiEntryEntityCount::default(),
    ));
}

fn add_perf_ui_entry<T: Default + Component>(
    mut commands: Commands,
    root: Query<Entity, With<PerfUiRoot>>,
) {
    commands.entity(root.single()).try_insert(T::default());
}

fn remove_perf_ui_entry<T: Component>(
    mut commands: Commands,
    root: Query<Entity, With<PerfUiRoot>>,
) {
    commands.entity(root.single()).remove::<T>();
}
