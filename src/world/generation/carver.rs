use ultraviolet::UVec2;

use crate::world::chunk::Chunk;

pub struct TerrainTransformer {
    pub influence: UVec2,
    pub spacing: UVec2,
    pub closure: Box<dyn Fn(&mut Chunk)>
}

impl TerrainTransformer {
    pub fn new(influence: UVec2, spacing: UVec2, closure: impl Fn(&mut Chunk) + 'static) -> Self {
        Self { influence, spacing, closure: Box::new(closure) }
    }

    pub fn apply(&self, chunk: &mut Chunk) {
        self.closure.call((chunk,))
    }
}