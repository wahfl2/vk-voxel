use ultraviolet::UVec3;

use super::{block_access::BlockAccess, block_data::BlockHandle};

pub struct Section {
    blocks: [[[BlockHandle; 16]; 16]; 16],
}

impl BlockAccess for Section {
    fn get_block(&self, pos: UVec3) -> BlockHandle {
        self.blocks[pos.x as usize][pos.y as usize][pos.z as usize]
    }

    fn set_block(&mut self, pos: UVec3, block: BlockHandle) {
        self.blocks[pos.x as usize][pos.y as usize][pos.z as usize] = block;
    }
}

impl Section {
    pub fn empty() -> Self {
        Self {
            blocks: [[[BlockHandle::default(); 16]; 16]; 16]
        }
    }

    pub fn flat_iter(&self) -> impl Iterator<Item = (UVec3, &BlockHandle)> {
        self.blocks.iter().enumerate()
            .flat_map(|(x, b)| { b.iter().enumerate()
                .flat_map(move |(y, b)| { b.iter().enumerate()
                    .map(move |(z, b)| { (UVec3::new(x as u32, y as u32, z as u32), b) }) }
                ) }
            )
    }

    pub fn get_column(&self, x: usize, z: usize) -> [&BlockHandle; 16] {
        let x_plane = self.blocks.get(x).unwrap();
        let column: Vec<&BlockHandle> = x_plane.iter().map(
            |f| { f.get(z).unwrap() }
        ).collect();

        column[..16].try_into().unwrap()
    }

    pub fn column_iter_mut(&mut self, x: usize, z: usize) -> impl Iterator<Item = &mut BlockHandle> {
        let x_plane = self.blocks.get_mut(x).unwrap();
        x_plane.iter_mut().map(move |p| { p.get_mut(z).unwrap() })
    }
}