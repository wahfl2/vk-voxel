use ultraviolet::{IVec2, UVec3};

use super::{section::Section, block_access::BlockAccess, block_data::BlockHandle};

pub struct Chunk {
    pub pos: IVec2,
    sections: [Section; 16],
}

impl BlockAccess for Chunk {
    fn get_block(&self, pos: UVec3) -> BlockHandle {
        let relative_y = pos.y % 16;
        let section_idx = (pos.y / 16) as usize;
        self.sections[section_idx].get_block(UVec3::new(pos.x, relative_y, pos.z))
    }
}

impl Chunk {
    
}