use std::{mem::size_of, collections::VecDeque};

use bytemuck::{Zeroable, Pod};
use ndarray::{Array3, Axis};
use ultraviolet::UVec3;
use vulkano::{memory::allocator::StandardMemoryAllocator, buffer::{BufferUsage, subbuffer::BufferWriteGuard, Subbuffer}};

use crate::{render::buffer::swap_buffer::{SwapBuffer, SwapDirtyPhase}, util::more_vec::UsizeVec3};

use super::{brickmap::BrickmapPointerRaw, bitmap::{Bitmap2, Bitmap, Bitmap4}};

pub const BRICKGRID_SIZE: [u32; 3] = [1024, 32, 1024];

pub const BRICKGRID_SIZE_X: u32 = BRICKGRID_SIZE[0];
pub const BRICKGRID_SIZE_Y: u32 = BRICKGRID_SIZE[1];
pub const BRICKGRID_SIZE_Z: u32 = BRICKGRID_SIZE[2];

const BGS_X: usize = BRICKGRID_SIZE_X as usize;
const BGS_Y: usize = BRICKGRID_SIZE_Y as usize;
const BGS_Z: usize = BRICKGRID_SIZE_Z as usize;

const _DATA_SIZE: usize = size_of::<BrickmapPointerRaw>() * BGS_X * BGS_Y * BGS_Z + 16;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Brickgrid {
    pub size: [u32; 3],
    pub _pad: u32,
    pub pointers: [[[BrickmapPointerRaw; BGS_Z]; BGS_Y]; BGS_X],
}

impl Brickgrid {
    pub fn new_empty(size: [u32; 3]) -> Self {
        Self {
            size,
            pointers: [[[BrickmapPointerRaw::zeroed(); BGS_X]; BGS_Y]; BGS_Z],
            _pad: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum BrickgridBufferTask {
    Array {
        offset_idx: UVec3,
        data: Array3<BrickmapPointerRaw>,
    },
    One {
        pos: UVec3,
        section: BrickmapPointerRaw,
    }
}

pub struct BrickgridBuffer {
    queue: VecDeque<BrickgridBufferTask>,
    after_free: VecDeque<BrickgridBufferTask>,
    pub inner: SwapBuffer<Brickgrid>,
    pub bitmap_2: SwapBuffer<Bitmap2>,
    pub bitmap_4: SwapBuffer<Bitmap4>,
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

        let bitmap_2 = SwapBuffer::from_data(
            BufferUsage::STORAGE_BUFFER, allocator, Bitmap2::new()
        );

        let bitmap_4 = SwapBuffer::from_data(
            BufferUsage::STORAGE_BUFFER, allocator, Bitmap4::new()
        );

        Self {
            queue: VecDeque::new(),
            after_free: VecDeque::new(),
            inner,
            bitmap_2,
            bitmap_4,
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
                if let Ok(mut bg_write) = free.write() {
                    let bm_free = self.bitmap_2.free_buffer();
                    let mut bm2_write = bm_free.write().unwrap();
                    let bm_free = self.bitmap_4.free_buffer();
                    let mut bm4_write = bm_free.write().unwrap();

                    // (there may be a better way to transfer the data instead of copying it)
                    self.after_free = self.queue.clone();
                    Self::write_queue_buffer(
                        &mut self.queue, 
                        &mut bg_write, 
                        &mut bm2_write,
                        &mut bm4_write
                    );

                    self.inner.swap();
                    self.dirty = SwapDirtyPhase::SwappedWaiting;
                    return true
                };
            },
            SwapDirtyPhase::SwappedWaiting => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.inner.free_buffer();
                if let Ok(mut bg_write) = free.write() {
                    let bm_free = self.bitmap_2.free_buffer();
                    let mut bm2_write = bm_free.write().unwrap();
                    let bm_free = self.bitmap_4.free_buffer();
                    let mut bm4_write = bm_free.write().unwrap();

                    // Write the new data that was written to the in-use buffer
                    Self::write_queue_buffer(
                        &mut self.after_free, 
                        &mut bg_write, 
                        &mut bm2_write,
                        &mut bm4_write
                    );

                    self.dirty = SwapDirtyPhase::Clean;
                    return true
                };
            },
            SwapDirtyPhase::SwappedImmediate => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.inner.free_buffer();
                if let Ok(mut bg_write) = free.write() {
                    let bm_free = self.bitmap_2.free_buffer();
                    let mut bm2_write = bm_free.write().unwrap();
                    let bm_free = self.bitmap_4.free_buffer();
                    let mut bm4_write = bm_free.write().unwrap();

                    // Write the new data that was written to the in-use buffer
                    Self::write_queue_buffer(
                        &mut self.after_free, 
                        &mut bg_write, 
                        &mut bm2_write,
                        &mut bm4_write
                    );

                    self.after_free = self.queue.clone();

                    // Write the new data to the now-free buffer and swap immediately
                    Self::write_queue_buffer(
                        &mut self.queue, 
                        &mut bg_write, 
                        &mut bm2_write,
                        &mut bm4_write
                    );

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
    fn write_queue_buffer(
        queue: &mut VecDeque<BrickgridBufferTask>, 
        brickgrid_wlock: &mut BufferWriteGuard<Brickgrid>,
        bitmap_2_wlock: &mut BufferWriteGuard<Bitmap2>,
        bitmap_4_wlock: &mut BufferWriteGuard<Bitmap4>,
    ) {
        let ptrs = &mut brickgrid_wlock.pointers;
        for task in queue.drain(..) {
            match task {
                BrickgridBufferTask::Array { offset_idx, data } => {
                    let task_size = UsizeVec3::new(
                        data.len_of(Axis(0)), 
                        data.len_of(Axis(1)), 
                        data.len_of(Axis(2))
                    );
                    
                    data.lanes(Axis(2)).into_iter().enumerate()
                        .for_each(|(i, lane)| {
                            let lane = lane.as_slice().unwrap();
                            let x = offset_idx.x + (i / task_size.y) as u32;
                            let y = offset_idx.y + (i % task_size.y) as u32;
                            let z = offset_idx.z as usize;
        
                            ptrs[x as usize][y as usize][z..z + lane.len()].copy_from_slice(lane);
                        }
                    );
                },

                BrickgridBufferTask::One { pos, section } => {
                    ptrs[pos.x as usize][pos.y as usize][pos.z as usize] = section;
                    bitmap_2_wlock.set(pos, section.pointer < 2);
                    bitmap_4_wlock.set(pos, section.pointer < 2);
                }
            }
        }
    }
}