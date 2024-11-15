use bevy::ecs::system::lifetimeless::SRes;
use bevy::prelude::*;
use iyes_perf_ui::entry::PerfUiEntry;

use crate::voxel::world::VoxelWorld;

#[derive(Component, Default)]
pub struct PerfUiEntryLoadedChunkCount;

impl PerfUiEntry for PerfUiEntryLoadedChunkCount {
    type SystemParam = SRes<VoxelWorld>;

    type Value = usize;

    fn label(&self) -> &str {
        "Loaded chunks"
    }

    fn sort_key(&self) -> i32 {
        10
    }

    fn update_value(
        &self,
        param: &mut <Self::SystemParam as bevy::ecs::system::SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Value> {
        Some(param.loaded_chunks.len())
    }
}
