use ultraviolet::UVec3;

use super::block_data::BlockHandle;

pub trait BlockAccess {
    /// Get a block relative to whatever this is being called on.
    /// 
    /// Will panic if OOB, might change that later.
    fn get_block(&self, pos: UVec3) -> BlockHandle;
}