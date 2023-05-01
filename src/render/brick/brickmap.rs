use std::mem;

use bytemuck::{Zeroable, Pod};

use crate::render::texture::TextureHandle;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct Brickmap {
    pub solid_mask: [[u8; 8]; 8],
    pub textures_offset: u32,
    pub lod_color: [u8; 3],
    pub _pad: u8,
}

impl Brickmap {
    pub fn empty() -> Self {
        Self {
            solid_mask: [[0; 8]; 8],
            textures_offset: 0,
            lod_color: [0; 3],
            _pad: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        for arr in self.solid_mask {
            for i in arr {
                if i > 0 { return false }
            }
        }
        true
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct BrickmapPointerRaw {
    pub pointer: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum BrickmapPointer {
    NotLoaded,
    Empty,
    Lod([u8; 3]),
    Brickmap(u32),
}

impl BrickmapPointer {
    pub fn to_raw(self) -> BrickmapPointerRaw {
        // Flags are in the least significant bits
        //
        // 00 => Not loaded in any capacity
        // 01 => Loaded, empty
        // 10 => Semi-loaded, next bits are the LOD color
        // 11 => Loaded, 30 most significant bits are index into brickmap buffer
        let pointer = match self {
            BrickmapPointer::NotLoaded => 0,
            BrickmapPointer::Empty => 1,
            BrickmapPointer::Lod([r, g, b]) => {
                let arr = [0, r, g, b];
                let trans = unsafe { mem::transmute::<_, u32>(arr) };
                trans << 2 | 0b10
            },
            BrickmapPointer::Brickmap(idx) => {
                if (idx >> 30) > 0 {
                    panic!("Brickmap index too large: {}", idx);
                }

                (idx << 2) | 0b11
            },
        };

        BrickmapPointerRaw { pointer }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct TexturePointer {
    pub index: u32,
}

impl From<TextureHandle> for TexturePointer {
    fn from(value: TextureHandle) -> Self {
        TexturePointer { index: value.index() }
    }
}