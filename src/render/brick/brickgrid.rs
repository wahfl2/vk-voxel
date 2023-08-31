use std::{mem::size_of, collections::VecDeque};

use bytemuck::{Zeroable, Pod};
use ultraviolet::UVec3;
use vulkano::buffer::subbuffer::BufferWriteGuard;

use super::brickmap::BrickmapPointerRaw;

pub const BRICKGRID_SIZE: [u32; 3] = [1024, 32, 1024];

const BGS_X: usize = BRICKGRID_SIZE[0] as usize;
const BGS_Y: usize = BRICKGRID_SIZE[1] as usize;
const BGS_Z: usize = BRICKGRID_SIZE[2] as usize;

pub const BG_ARRAY_SIZE: usize = BGS_X * BGS_Y * BGS_Z;

const _DATA_SIZE: usize = size_of::<BrickmapPointerRaw>() * BGS_X * BGS_Y * BGS_Z;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Brickgrid {
    pub pointers: [BrickmapPointerRaw; BG_ARRAY_SIZE],
}

impl Brickgrid {
    pub fn new_empty() -> Self {
        Self {
            pointers: [BrickmapPointerRaw::zeroed(); BG_ARRAY_SIZE],
        }
    }
}

#[derive(Clone, Debug)]
pub enum BrickgridBufferTask {
    One {
        pos: UVec3,
        section: BrickmapPointerRaw,
    }
}

/// Writes a queue to a write lock and clears the queue.
pub fn write_queue_buffer(queue: &mut VecDeque<BrickgridBufferTask>, write_lock: &mut BufferWriteGuard<Brickgrid>) {
    let ptrs = &mut write_lock.pointers;
    for task in queue.drain(..) {
        match task {
            BrickgridBufferTask::One { pos, section } => {
                ptrs[morton_encode(pos.x, pos.y, pos.z)] = section;
            }
        }
    }
}

// XZ XYZ XZ XYZ XZ XYZ XZ XYZ XZ XYZ
pub fn morton_encode(x: u32, y: u32, z: u32) -> usize {
    let mut i = 0;

    for b in 0..5u32 {
        let mask = 1 << b;

        let b2 = b * 2;
        let mask2 = 1 << b2;

        let b21 = b2 + 1;
        let mask21 = 1 << b21;

        let zb = (z & mask2) >> b2;
        let yb = (y & mask) >> b;
        let xb = (x & mask2) >> b2;
        let zb2 = (z & mask21) >> b21;
        let xb2 = (x & mask21) >> b21;

        let append = zb | (yb << 1) | (xb << 2) | (zb2 << 3) | (xb2 << 4);
        i |= append << (b * 5);
    }
    
    i as usize
}