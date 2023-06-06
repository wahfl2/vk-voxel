use bytemuck::{Pod, Zeroable};
use ultraviolet::UVec3;

use super::brickgrid::{BRICKGRID_SIZE_Z, BRICKGRID_SIZE_X, BRICKGRID_SIZE_Y};

const BGS_X: usize = BRICKGRID_SIZE_X as usize;
const BGS_Y: usize = BRICKGRID_SIZE_Y as usize;
const BGS_Z: usize = BRICKGRID_SIZE_Z as usize;

pub trait Bitmap {
    fn set(&mut self, offset: UVec3, empty: bool);
}

macro_rules! bitmap {
    ($struct_name:ident,$bit_size:expr) => {
        #[repr(C)]
        #[derive(Clone, Copy, Debug, Pod, Zeroable)]
        pub struct $struct_name {
            pub size: [u32; 3],
            pub bit_size: u32,
            pub bits: [[[u8; $struct_name::BMS_Z]; $struct_name::BMS_Y]; $struct_name::BMS_X]
        }

        impl $struct_name {
            const BMS_X: usize = BGS_X / $bit_size;
            const BMS_Y: usize = BGS_Y / $bit_size;
            const BMS_Z: usize = BGS_Z / $bit_size / 8;

            const _DIV_CHECK: () = assert!(
                BGS_X % $bit_size == 0 &&
                BGS_Y % $bit_size == 0 &&
                BGS_Z % $bit_size == 0
            );

            pub fn new() -> Self {
                Self {
                    size: [$struct_name::BMS_X as u32, $struct_name::BMS_Y as u32, $struct_name::BMS_Z as u32 * 8],
                    bit_size: $bit_size,
                    bits: [[[0; $struct_name::BMS_Z]; $struct_name::BMS_Y]; $struct_name::BMS_X]
                }
            }
        }

        impl Bitmap for $struct_name {
            fn set(&mut self, offset: UVec3, empty: bool) {
                let idx = offset / $bit_size;
                let bit_idx = 7 - (idx.z % 8);
                let bit = if empty { 0 } else { 1 };

                let num = &mut self.bits[idx.x as usize][idx.y as usize][(idx.z / 8) as usize];
                *num &= !(1 << bit_idx);
                *num |= (bit << bit_idx);
            }
        }
    };
}

// Using a macro because const generics aren't good enough yet
bitmap!(Bitmap2, 2);
bitmap!(Bitmap4, 4);
bitmap!(Bitmap8, 8);
bitmap!(Bitmap16, 16);