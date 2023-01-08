use std::{sync::Arc, collections::hash_map::Iter};

use rustc_data_structures::stable_map::FxHashMap;
use ultraviolet::IVec2;
use vulkano::{buffer::{BufferUsage, CpuAccessibleBuffer}, device::Device, memory::allocator::FastMemoryAllocator};

use crate::render::vertex::VertexRaw;

use super::buffer_queue::BufferQueue;

pub struct VertexChunkBuffer {
    inner: Arc<CpuAccessibleBuffer<[VertexRaw]>>,
    inner_size: u64,
    allocator: Arc<FastMemoryAllocator>,
    chunk_allocator: ChunkBufferAllocator,
    allocations: FxHashMap<(i32, i32), ChunkBufferAllocation>,
    pub queue: BufferQueue,
}

impl VertexChunkBuffer {
    const INITIAL_SIZE: u64 = 1000;

    pub fn new(device: Arc<Device>) -> Self {
        let allocator = Arc::new(FastMemoryAllocator::new_default(device.clone()));

        VertexChunkBuffer {
            inner: Self::create_inner(allocator.clone(), Self::INITIAL_SIZE),
            inner_size: Self::INITIAL_SIZE,
            allocator,
            chunk_allocator: ChunkBufferAllocator::new(),
            allocations: FxHashMap::default(),
            queue: BufferQueue::new()
        }
    }

    pub fn push_chunk_vertices(&mut self, chunk_pos: IVec2, vertices: &[VertexRaw]) {
        let size = vertices.len() as u32 * std::mem::size_of::<VertexRaw>() as u32;
        let allocation = self.chunk_allocator.allocate(size);
        self.allocations.insert(chunk_pos.into(), allocation);
        if allocation.back as u64 >= self.inner_size {
            self.grow_inner();
        }
        self.queue.push_data(allocation.front, vertices);
    }

    pub fn remove_chunk(&mut self, chunk_pos: IVec2) {
        if let Some(allocation) = self.allocations.get(&chunk_pos.into()) {
            self.chunk_allocator.deallocate(allocation);
            // No need to push to queue, corresponding memory will not be read
        } else {
            panic!("Tried to remove chunk that was not allocated. Chunk pos: {:?}", chunk_pos);
        }
    }

    pub fn get_buffer(&self) -> Arc<CpuAccessibleBuffer<[VertexRaw]>> {
        self.inner.clone()
    }

    fn create_inner(
        allocator: Arc<FastMemoryAllocator>, 
        size: u64
    ) -> Arc<CpuAccessibleBuffer<[VertexRaw]>> {
        unsafe {
            CpuAccessibleBuffer::uninitialized_array(
                &allocator, 
                size, 
                BufferUsage {
                    vertex_buffer: true,
                    ..Default::default()
                }, 
                false
            ).unwrap()
        }
    }

    fn grow_inner(&mut self) {
        let old_buffer = self.inner.clone();
        self.inner = Self::create_inner(self.allocator.clone(), self.inner_size * 2);

        let write = &mut self.inner.write().unwrap();
        write[0..self.inner_size as usize].copy_from_slice(&old_buffer.read().unwrap());
        self.inner_size *= 2;
    }
}

/// Describes an allocation in the chunk vertex buffer. Has no real spot in the memory backing it.
#[derive(Copy, Clone, Debug)]
pub struct ChunkBufferAllocation {
    pub front: u32,
    pub back: u32,
}

impl ChunkBufferAllocation {
    fn new(front: u32, back: u32) -> Self {
        Self { front, back }
    }

    fn new_size(front: u32, size: u32) -> Self {
        Self::new(front, front + size)
    }
}

struct ChunkBufferAllocator {
    /// Represents all free sections
    dual_map: DualHashMap<u32>,
    top: u32,
}

impl ChunkBufferAllocator {
    fn new() -> Self {
        Self {
            dual_map: DualHashMap::new(),
            top: 0,
        }
    }

    fn allocate(&mut self, size: u32) -> ChunkBufferAllocation {
        if let Some((front, back, free_size)) = Self::find_free(&self.dual_map, size) {
            self.dual_map.remove_front(&front);
            if free_size != size { self.dual_map.insert(front + size, back); }
            ChunkBufferAllocation::new_size(front, size)
        } else {
            self.dual_map.insert(self.top, self.top + size);
            let old_top = self.top;
            self.top += size;
            ChunkBufferAllocation::new_size(old_top, size)
        }
    }

    /// Deallocates a section and marks it as free.<br>
    /// <b>DOES NOT CHECK IF THIS ALLOCATION IS VALID</b>. *Use an allocation generated by* `.allocate()`
    fn deallocate(&mut self, alloc: &ChunkBufferAllocation) {
        let mut joined_alloc = alloc.clone();

        if let Some(back) = self.dual_map.remove_front(&alloc.back) {
            joined_alloc.back = back.to_owned();
        }

        if let Some(front) = self.dual_map.remove_back(&alloc.front) {
            joined_alloc.front = front.to_owned();
        }

        self.dual_map.insert(joined_alloc.front, joined_alloc.back);
    }

    fn find_free(map: &DualHashMap<u32>, needed_size: u32) -> Option<(u32, u32, u32)> {
        for (front, back) in map.iter() {
            let free_size = back - front;
            if needed_size <= free_size {
                return Some((front.to_owned(), back.to_owned(), free_size))
            }
        }
        None
    }
}

struct DualHashMap<T> {
    front_back: FxHashMap<T, T>,
    back_front: FxHashMap<T, T>,
}

impl<T> DualHashMap<T> 
where
    T: std::hash::Hash + std::cmp::Eq + Clone
{
    fn new() -> Self {
        Self {
            front_back: FxHashMap::default(),
            back_front: FxHashMap::default(),
        }
    }

    fn insert(&mut self, front: T, back: T) {
        self.front_back.insert(front.clone(), back.clone());
        self.back_front.insert(back, front);
    }

    fn get_back(&self, front: &T) -> Option<&T> {
        self.front_back.get(front)
    }

    fn get_front(&self, back: &T) -> Option<&T> {
        self.back_front.get(back)
    }

    /// Removes the pair with the specified front, returning the back if it existed.
    fn remove_front(&mut self, front: &T) -> Option<T> {
        match self.front_back.remove(front) {
            Some(back) => {
                self.back_front.remove(&back);
                return Some(back)
            },
            None => return None
        }        
    }

    /// Removes the pair with the specified back, returning the front if it existed.
    fn remove_back(&mut self, back: &T) -> Option<T> {
        match self.back_front.remove(back) {
            Some(front) => {
                self.front_back.remove(&front);
                return Some(front)
            },
            None => return None
        }   
    }

    fn iter(&self) -> Iter<'_, T, T> {
        self.front_back.iter()
    }
}