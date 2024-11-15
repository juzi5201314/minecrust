use bevy::prelude::IVec3;
use noise::{HybridMulti, NoiseFn, Perlin};

use crate::voxel::voxel_block::{BlockId, AIR};

use super::Generator;

pub struct NoiseGenerator {
    noise: HybridMulti<Perlin>,
    cache: papaya::HashMap<(i32, i32), f64, ahash::RandomState>,
}

impl NoiseGenerator {
    pub fn new() -> Self {
        let mut noise = HybridMulti::<Perlin>::new(1234);
        noise.octaves = 5;
        noise.frequency = 1.1;
        noise.lacunarity = 2.8;
        noise.persistence = 0.4;
        let cache = papaya::HashMap::default();

        NoiseGenerator { noise, cache }
    }
}

impl Generator for NoiseGenerator {
    fn generate(&self, pos: IVec3) -> BlockId {
        let [x, y, z] = pos.as_dvec3().to_array();

        let sample = *self.cache.pin().get_or_insert_with((pos.x, pos.z), || {
            self.noise.get([x / 1000.0, z / 1000.0]) * 50.0
        });

        // If y is less than the noise sample, we will set the voxel to solid
        let is_ground = y < sample;

        if is_ground {
            BlockId::new("core::grass")
        } else {
            AIR.clone()
        }
    }
}
