use std::{mem::size_of, collections::VecDeque};

use bytemuck::{Zeroable, Pod};
use ndarray::{Array3, Axis};
use ultraviolet::UVec3;
use vulkano::{memory::allocator::StandardMemoryAllocator, buffer::{BufferUsage, subbuffer::BufferWriteGuard, Subbuffer}};

use crate::{render::buffer::swap_buffer::{SwapBuffer, SwapDirtyPhase}, util::more_vec::UsizeVec3};

use super::brickmap::BrickmapPointerRaw;

pub const BRICKGRID_SIZE: [u32; 3] = [1024, 32, 1024];

const BGS_X: usize = BRICKGRID_SIZE[0] as usize;
const BGS_Y: usize = BRICKGRID_SIZE[1] as usize;
const BGS_Z: usize = BRICKGRID_SIZE[2] as usize;

const BG_ARRAY_SIZE: usize = BGS_X * BGS_Y * BGS_Z;

const _DATA_SIZE: usize = size_of::<BrickmapPointerRaw>() * BGS_X * BGS_Y * BGS_Z + 16;

// XZ XYZ XZ XYZ XZ XYZ XZ XYZ XZ XYZ
fn morton_encode(x: u32, y: u32, z: u32) -> usize {
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Brickgrid {
    pub size: [u32; 3],
    pub _pad: u32,
    pub pointers: [BrickmapPointerRaw; BG_ARRAY_SIZE],
}

impl Brickgrid {
    pub fn new_empty(size: [u32; 3]) -> Self {
        Self {
            size,
            pointers: [BrickmapPointerRaw::zeroed(); BG_ARRAY_SIZE],
            _pad: 0,
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

pub struct BrickgridBuffer {
    queue: VecDeque<BrickgridBufferTask>,
    after_free: VecDeque<BrickgridBufferTask>,
    pub inner: SwapBuffer<Brickgrid>,
    pub dirty: SwapDirtyPhase,
}

// Maybe there's a more versatile swapping buffer solution
impl BrickgridBuffer {
    pub fn new(allocator: &StandardMemoryAllocator) -> Self {
        let inner: SwapBuffer<Brickgrid> = SwapBuffer::new_sized(BufferUsage::STORAGE_BUFFER, allocator);
        let mut w = inner.buffer_1.write().unwrap();
        w.size = BRICKGRID_SIZE;
        drop(w);

        let mut w = inner.buffer_2.write().unwrap();
        w.size = BRICKGRID_SIZE;
        drop(w);

        Self {
            queue: VecDeque::new(),
            after_free: VecDeque::new(),
            inner,
            dirty: SwapDirtyPhase::Clean,
        }
    }

    pub fn write(&mut self, task: BrickgridBufferTask) {
        self.queue.push_back(task);
        match self.dirty {
            SwapDirtyPhase::SwappedWaiting => self.dirty = SwapDirtyPhase::SwappedImmediate,
            SwapDirtyPhase::Clean => self.dirty = SwapDirtyPhase::Dirty,
            _ => ()
        }
    }

    pub fn update(&mut self) -> bool {
        match self.dirty {
            SwapDirtyPhase::Dirty => {
                let free = self.inner.free_buffer();
                if let Ok(mut write) = free.write() {
                    // (there may be a better way to transfer the data instead of copying it)
                    self.after_free = self.queue.clone();
                    Self::write_queue_buffer(&mut self.queue, &mut write);

                    self.inner.swap();
                    self.dirty = SwapDirtyPhase::SwappedWaiting;
                    return true
                };
            },
            SwapDirtyPhase::SwappedWaiting => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.inner.free_buffer();
                if let Ok(mut write) = free.write() {
                    // Write the new data that was written to the in-use buffer
                    Self::write_queue_buffer(&mut self.after_free, &mut write);
                    self.dirty = SwapDirtyPhase::Clean;
                    return true
                };
            },
            SwapDirtyPhase::SwappedImmediate => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.inner.free_buffer();
                if let Ok(mut write) = free.write() {
                    // Write the new data that was written to the in-use buffer
                    Self::write_queue_buffer(&mut self.after_free, &mut write);
                    self.after_free = self.queue.clone();

                    // Write the new data to the now-free buffer and swap immediately
                    Self::write_queue_buffer(&mut self.queue, &mut write);
                    self.inner.swap();
                    self.dirty = SwapDirtyPhase::SwappedWaiting;
                    return true
                };
            },
            SwapDirtyPhase::Clean => (),
        }
        false
    }

    pub fn get_buffer(&self) -> Subbuffer<Brickgrid> {
        self.inner.current_buffer()
    }

    /// Writes a queue to a write lock and clears the queue.
    fn write_queue_buffer(queue: &mut VecDeque<BrickgridBufferTask>, write_lock: &mut BufferWriteGuard<Brickgrid>) {
        let ptrs = &mut write_lock.pointers;
        for task in queue.drain(..) {
            match task {
                BrickgridBufferTask::One { pos, section } => {
                    ptrs[morton_encode(pos.x, pos.y, pos.z)] = section;
                }
            }
        }
    }
}