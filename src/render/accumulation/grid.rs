use vulkano::buffer::BufferContents;

use crate::render::brick::brickgrid::BRICKGRID_SIZE;

const SGS_X: usize = BRICKGRID_SIZE[0] as usize;
const SGS_Y: usize = BRICKGRID_SIZE[1] as usize;
const SGS_Z: usize = BRICKGRID_SIZE[2] as usize;

const SG_ARRAY_SIZE: usize = SGS_X * SGS_Y * SGS_Z;

#[repr(C)]
#[derive(Clone, BufferContents)]
pub struct SurfelGrid {
    // surfel map slices
    pub pointers: [u32; SG_ARRAY_SIZE],
}

#[derive(Copy, Clone, Debug)]
pub enum SurfelMapSlice {
    // one flag bit
    Empty,
    SurfelMap {
        // must be < 18 bits
        start: u32,
        // must be < 13 bits
        length: u32,
    }
}

impl SurfelMapSlice {
    pub fn from_raw(pointer: u32) -> Self {
        match pointer & 1 {
            0 => Self::Empty,
            1 => {
                Self::SurfelMap { 
                    start: (pointer >> 1) & 0b1111_1111_1111_1111_11, 
                    length: pointer >> 19,
                }
            },
            _ => unreachable!(),
        }
    }

    pub fn to_raw(&self) -> u32 {
        match *self {
            Self::Empty => 0,
            Self::SurfelMap { start, length } => {
                if start > 0b1111_1111_1111_1111_11 || length > 0b1111_1111_1111_1 {
                    panic!("Too many surfel!!");
                }

                1 | (start << 1) | (length << 19)
            }
        }
    }
}