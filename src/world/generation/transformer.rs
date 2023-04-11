use std::collections::hash_map::Entry;

use ahash::{HashMap, HashMapExt};
use ultraviolet::{UVec2, IVec2};

use crate::util::util::UVecToSigned;

use super::terrain::TerrainChunk;

pub struct TerrainTransformer<T> {
    pub size: UVec2,
    pub spacing: UVec2,
    data_cache: HashMap<IVec2, T>,

    pub data_generator: Box<dyn FnMut(IVec2, UVec2) -> T>,
    pub closure: Box<dyn FnMut(&mut TerrainChunk, IVec2, UVec2, &T)>
}

impl<T> TerrainTransformer<T>
where T: Clone
{
    pub fn new(
        size: UVec2, 
        spacing: UVec2, 
        data_generator: impl FnMut(IVec2, UVec2) -> T + 'static,
        closure: impl FnMut(&mut TerrainChunk, IVec2, UVec2, &T) + 'static
    ) -> Self {
        assert!(size.x > 0 && size.y > 0, "Size must be larger than 0");
        assert!(spacing.x > 0 && spacing.y > 0, "Spacing must be larger than 0");

        Self { 
            size, 
            spacing, 
            data_cache: HashMap::new(),
            data_generator: Box::new(data_generator),
            closure: Box::new(closure),
        }
    }

    pub fn apply(&mut self, chunk: &mut TerrainChunk) {
        let locations = self.locations(chunk.blocks.pos);
        let offsets = locations.clone().into_iter().map(|v| { v * self.spacing.signed() }).collect::<Vec<_>>();
        let pos = chunk.blocks.pos;
        println!("chunk ({}, {}), offsets: {:?}", pos.x, pos.y, offsets);

        for location in locations {
            let data = &*match self.data_cache.entry(location) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(self.data_generator.call_mut((location, self.size))),
            };

            let offset = location * self.spacing.signed();
            self.closure.call_mut((chunk, offset, self.size, data))
        }
    }

    fn locations(&self, chunk_pos: IVec2) -> Vec<IVec2> {
        let minus_1 = self.size - UVec2::one();
        let iv = IVec2::new(minus_1.x as i32, minus_1.y as i32);
        let check_min = chunk_pos - iv;

        let mut ret = Vec::new();
        let i_spacing = self.spacing.signed();

        for chunk_x in check_min.x..=chunk_pos.x {
            if (chunk_x % i_spacing.x) == 0 {
                for chunk_y in check_min.y..=chunk_pos.y {
                    if (chunk_y % i_spacing.y) == 0 {
                        ret.push(IVec2::new(chunk_x, chunk_y) / i_spacing);
                    }
                }
            }
        }

        ret
    }
}