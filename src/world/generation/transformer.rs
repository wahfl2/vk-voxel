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
        // let offsets = locations.clone().into_iter().map(|v| { v * self.spacing.signed() }).collect::<Vec<_>>();
        // let pos = chunk.blocks.pos;
        // println!("chunk ({}, {}), offsets: {:?}", pos.x, pos.y, offsets);

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
        let min = chunk_pos - (self.size - UVec2::one()).signed();
        let i_spacing = self.spacing.signed();
        let mut ret = Vec::new();
        for x in min.x..=chunk_pos.x {
            if x % i_spacing.x == 0 {
                for y in min.y..=chunk_pos.y {
                    if y % i_spacing.y == 0 {
                        ret.push(IVec2::new(x, y) / self.spacing.signed());
                    }
                }
            }
        }

        ret
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn transformer() {
        fn expect(transformer: TerrainTransformer<()>, expected: Vec<IVec2>, chunk_pos: IVec2) {
            let mut actual = transformer.locations(chunk_pos);
            for e in expected {
                let i = actual.index_of(&e).expect(&format!("Actual did not contain {:?},\nActual: {:?}", e, actual));
                actual.swap_remove(i);
            }
    
            if !actual.is_empty() {
                panic!("Actual contained extraneous elements: {:?}", actual);
            }
        }

        expect(
            TerrainTransformer::new(
                UVec2::new(2, 2),
                UVec2::new(1, 1),
                |_, _| {}, |_, _, _, _| {}
            ), 
            vec![
                IVec2::new(-1, -1),
                IVec2::new(0, -1),
                IVec2::new(-1, 0),
                IVec2::new(0, 0),
            ], 
            IVec2::zero(),
        );

        expect(
            TerrainTransformer::new(
                UVec2::new(3, 3),
                UVec2::new(1, 1),
                |_, _| {}, |_, _, _, _| {}
            ), 
            vec![
                IVec2::new(-2, -2), IVec2::new(-2, -1), IVec2::new(-2, 0),
                IVec2::new(-1, -2), IVec2::new(-1, -1), IVec2::new(-1, 0),
                IVec2::new(0, -2), IVec2::new(0, -1), IVec2::new(0, 0),
            ], 
            IVec2::zero(),
        );

        expect(
            TerrainTransformer::new(
                UVec2::new(3, 3),
                UVec2::new(2, 2),
                |_, _| {}, |_, _, _, _| {}
            ), 
            vec![
                IVec2::new(-1, -1), IVec2::new(-1, 0),
                IVec2::new(0, -1), IVec2::new(0, 0),
            ], 
            IVec2::zero(),
        );
    }

    trait IndexOf<T> {
        fn index_of(&self, x: &T) -> Option<usize>;
    }

    impl<T> IndexOf<T> for Vec<T> 
    where T: PartialEq
    {
        fn index_of(&self, x: &T) -> Option<usize> {
            for (i, e) in self.iter().enumerate() {
                if e == x { return Some(i) }
            }
            None
        }
    }
}