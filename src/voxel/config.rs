use bevy::prelude::Resource;

#[derive(Resource, Debug, Clone)]
pub struct VoxelConfig {
    pub spawning_distance: u32,
    pub spawning_rays: u32,
    pub spawning_ray_margin: u32,
    pub max_spawn_per_frame: u32,
}

impl Default for VoxelConfig {
    fn default() -> Self {
        Self {
            spawning_distance: 64,
            spawning_rays: 96,
            spawning_ray_margin: 24,
            max_spawn_per_frame: 8192,
        }
    }
}
