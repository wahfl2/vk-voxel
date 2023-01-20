use rustc_data_structures::stable_map::FxHashMap;
use ultraviolet::IVec2;

use super::chunk::Chunk;

pub struct World {
    pub loaded_chunks: FxHashMap<IVec2, Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self {
            loaded_chunks: FxHashMap::default(),
        }
    }

    pub fn load_chunk(&mut self, chunk_pos: IVec2) {
        
    }
}