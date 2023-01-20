use ultraviolet::UVec3;

use super::{block_access::BlockAccess, block_data::BlockHandle};

pub struct Section {
    blocks: [[[BlockHandle; 16]; 16]; 16],
}

impl BlockAccess for Section {
    fn get_block(&self, pos: UVec3) -> BlockHandle {
        self.blocks[pos.x as usize][pos.y as usize][pos.z as usize]
    }
}

impl Section {
    pub fn empty() -> Self {
        Self {
            blocks: [[[BlockHandle::default(); 16]; 16]; 16]
        }
    }
}