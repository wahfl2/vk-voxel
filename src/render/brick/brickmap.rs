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
    pub bb_min: u16,
    pub bb_max: u16,
}

impl Brickmap {
    pub fn empty() -> Self {
        Self {
            solid_mask: [[0; 8]; 8],
            textures_offset: 0,
            lod_color: [0; 3],
            _pad: 0,
            bb_min: pack_u4_vec3(0, 0, 0),
            bb_max: pack_u4_vec3(0, 0, 0)
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

    pub fn update_bounding_box(&mut self) {
        let (mut x_min, mut y_min, mut z_min) = (7, 7, 7);
        let (mut x_max, mut y_max, mut z_max) = (0, 0, 0);

        for (x, x_plane) in self.solid_mask.iter().enumerate() {
            let x = x as u16;

            for (y, y_column) in x_plane.iter().enumerate() {
                let y = y as u16;

                for z in 0..8u16 {
                    let shift = 7 - z;
                    if (y_column >> shift) & 1 > 0 {
                        (x_min, y_min, z_min) = (x_min.min(x), y_min.min(y), z_min.min(z));
                        (x_max, y_max, z_max) = (x_max.max(x), y_max.max(y), z_max.max(z));
                    }
                }
            }
        }

        self.bb_min = pack_u4_vec3(x_min, y_min, z_min);
        self.bb_max = pack_u4_vec3(x_max + 1, y_max + 1, z_max + 1);
    }
}

fn pack_u4_vec3(x: u16, y: u16, z: u16) -> u16 {
    ((x & 0b1111) << 12) | ((y & 0b1111) << 8) | ((z & 0b1111) << 4)
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
        //  00 => Not loaded in any capacity
        // 100 => Requested to be loaded
        //  01 => Loaded, empty
        //  10 => Semi-loaded, next bits are the LOD color
        //  11 => Loaded, 30 most significant bits are index into brickmap buffer
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