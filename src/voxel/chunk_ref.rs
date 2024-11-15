use std::sync::Arc;

use super::chunk::ChunkData;

#[derive(Clone)]
pub struct ChunkRef {
    pub refs: Vec<Arc<ChunkData>>,
}
