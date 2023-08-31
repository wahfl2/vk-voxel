use std::collections::{VecDeque, hash_map::Entry};

use ahash::{HashMap, HashMapExt};
use ultraviolet::{UVec3, IVec3};
use vulkano::{buffer::{subbuffer::BufferWriteGuard, Subbuffer, Buffer, BufferCreateInfo, BufferContents, BufferUsage}, memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage}};

use crate::{render::{brick::brickgrid::morton_encode, util::CreateInfoConvenience, buffer::allocator::{ChunkBufferAllocator, ChunkBufferAllocation}}, util::util::{VecRounding, UVecTrunc}};

use super::{grid::SurfelGrid, surfel::{SurfelRaw, Surfel}};

const MAX_SURFELS: usize = 0b1111_1111_1111_1111_11 + 1;

pub struct SurfelBuffers {
    pub surfel_buffer: SurfelBuffer,
    pub surfel_grid_buffer: Subbuffer<SurfelGrid>,
    pub surfel_map_buffer: Subbuffer<SurfelMap>,
}

impl SurfelBuffers {
    pub fn new(allocator: &StandardMemoryAllocator) -> Self {
        fn create_buffer<T: BufferContents>(allocator: &StandardMemoryAllocator) -> Subbuffer<T> {
            Buffer::new_sized(
                allocator, 
                BufferCreateInfo::usage(BufferUsage::STORAGE_BUFFER), 
                AllocationCreateInfo::usage(MemoryUsage::Upload),
            ).unwrap()
        }

        Self {
            surfel_buffer: SurfelBuffer::new(allocator),
            surfel_grid_buffer: create_buffer(allocator),
            surfel_map_buffer: create_buffer(allocator)
        }
    }

    pub fn insert_surfel(&mut self, surfel: Surfel) {
        let surfel_index = self.surfel_buffer.insert(surfel.to_raw());

        const RECIP_16: f32 = 1.0 / 16.0;
        let section_pos = (surfel.world_pos * RECIP_16).floor().into_u();
        
    }
}

pub struct SurfelBuffer {
    free: VecDeque<usize>,
    top: usize,
    inner: Subbuffer<SurfelContainer>
}

impl SurfelBuffer {
    pub fn new(allocator: &StandardMemoryAllocator) -> Self {
        Self {
            free: VecDeque::new(),
            top: 0,
            inner: Buffer::new_sized(
                allocator, 
                BufferCreateInfo::usage(BufferUsage::STORAGE_BUFFER), 
                AllocationCreateInfo::usage(MemoryUsage::Upload),
            ).unwrap()
        }
    }

    pub fn insert(&mut self, surfel: SurfelRaw) -> u32 {
        let idx = match self.free.pop_front() {
            Some(idx) => idx,
            None => {
                self.top += 1;
                self.top - 1
            },
        };

        let mut write = self
            .inner
            .write()
            .expect("Insert should only be called when free.");
        write.surfels[idx] = surfel;

        idx as u32
    }

    pub fn remove(&mut self, index: u32) {
        let mut write = self
            .inner
            .write()
            .expect("Remove should only be called when free.");
        write.surfels[index as usize] = SurfelRaw::default();
    }
}

#[repr(C)]
#[derive(BufferContents)]
pub struct SurfelContainer {
    pub surfel_count: u32,
    pub surfels: [SurfelRaw; MAX_SURFELS]
}

pub struct SurfelMapper {
    buffer: Subbuffer<SurfelMap>,
    allocator: ChunkBufferAllocator,
    pub allocations: HashMap<IVec3, ChunkBufferAllocation>,
    pub highest: usize,
}

impl SurfelMapper {
    pub fn new(allocator: &StandardMemoryAllocator) -> Self {
        Self {
            buffer: Buffer::new_sized(
                allocator, 
                BufferCreateInfo::usage(BufferUsage::STORAGE_BUFFER), 
                AllocationCreateInfo::usage(MemoryUsage::Upload),
            ).unwrap(),
            allocator: ChunkBufferAllocator::new(),
            allocations: HashMap::new(),
            highest: 0,
        }
    }

    pub fn add_surfel_to_section(&mut self, surfel_index: u32, section_pos: IVec3) {
        let mut write = self.buffer.write().expect("Add should only be called when free.");
        match self.allocations.entry(section_pos) {
            Entry::Occupied(mut entry) => {
                let alloc = *entry.get();
                self.allocator.deallocate(&alloc);
                let new_alloc = self.allocator.allocate((alloc.back - alloc.front) + 1);
                entry.insert(new_alloc);

                let (old_front, old_back) = (alloc.front as usize, alloc.back as usize);
                let (new_front, new_back) = (new_alloc.front as usize, new_alloc.back as usize);

                write.pointers.copy_within(old_front..old_back, new_front);
                write.pointers[new_back - 1] = surfel_index;
            },
            Entry::Vacant(entry) => {
                let new_alloc = self.allocator.allocate(1);
                entry.insert(new_alloc);
                write.pointers[new_alloc.front as usize] = surfel_index;
            },
        }
    }
}

#[repr(C)]
#[derive(BufferContents)]
pub struct SurfelMap {
    // indices for the surfel container, contiguous for each section
    pub pointers: [u32; MAX_SURFELS]
}

impl SurfelMap {
    pub fn new() -> Self {
        Self {
            pointers: [0; MAX_SURFELS]
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfelGridBufferTask {
    pub pos: UVec3,
    pub ptr: u32,
}

/// Writes a queue to a write lock and clears the queue.
pub fn write_queue_buffer(queue: &mut VecDeque<SurfelGridBufferTask>, write_lock: &mut BufferWriteGuard<SurfelGrid>) {
    let ptrs = &mut write_lock.pointers;
    for task in queue.drain(..) {
        let pos = task.pos;
        ptrs[morton_encode(pos.x, pos.y, pos.z)] = task.ptr;
    }
}