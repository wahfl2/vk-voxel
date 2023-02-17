use std::{sync::Arc, collections::hash_map::Iter};

use rustc_data_structures::stable_map::FxHashMap;

use ultraviolet::IVec2;
use vulkano::{buffer::{CpuAccessibleBuffer, BufferContents, BufferUsage, DeviceLocalBuffer}, device::Device, memory::allocator::StandardMemoryAllocator, command_buffer::{DrawIndirectCommand, AutoCommandBufferBuilder, PrimaryAutoCommandBuffer}};

use super::swap_buffer::SwappingBuffer;

pub struct HeapBuffer<T> 
where [T]: BufferContents,
{
    buffer: SwappingBuffer<T>,
    chunk_allocator: ChunkBufferAllocator,
    pub allocations: FxHashMap<(i32, i32), ChunkBufferAllocation>,
    pub indirect_buffer: Option<Arc<DeviceLocalBuffer<[DrawIndirectCommand]>>>,
    pub vertex_count_multiplier: u32,
    pub highest: usize,
}

impl<U> HeapBuffer<U> 
where 
    U: Copy + Clone,
    [U]: BufferContents,
{
    const INITIAL_SIZE: usize = 10_000_000;

    pub fn new(device: Arc<Device>, usage: BufferUsage, vertex_count_multiplier: u32) -> Self {
        let allocator = StandardMemoryAllocator::new_default(device.clone());

        HeapBuffer {
            buffer: SwappingBuffer::new(Self::INITIAL_SIZE, usage, &allocator),
            chunk_allocator: ChunkBufferAllocator::new(),
            allocations: FxHashMap::default(),
            indirect_buffer: None,
            vertex_count_multiplier,
            highest: 0
        }
    }

    pub fn update(&mut self, allocator: &StandardMemoryAllocator, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> bool {
        let swapped = self.buffer.update();
        if swapped {
            let data = self.get_ind_commands(self.vertex_count_multiplier);
            if data.len() > 0 {
                self.indirect_buffer = Some(DeviceLocalBuffer::<[DrawIndirectCommand]>::from_iter(
                    allocator, 
                    data, 
                    BufferUsage {
                        indirect_buffer: true,
                        ..Default::default()
                    }, 
                    builder
                ).unwrap());
            } else {
                self.indirect_buffer = None;
            }
        }
        swapped
    }

    pub fn insert(
        &mut self, 
        chunk_pos: IVec2, 
        data: &[U],
    ) {
        let size = data.len() as u32;
        let allocation = self.chunk_allocator.allocate(size);
        if allocation.back as usize > self.highest {
            self.highest = allocation.back as usize;
        }

        self.allocations.insert(chunk_pos.into(), allocation);
        self.buffer.write(allocation.front.try_into().unwrap(), data);
    }

    pub fn remove(&mut self, chunk_pos: IVec2) {
        if let Some(allocation) = self.allocations.remove(&chunk_pos.into()) {
            self.chunk_allocator.deallocate(&allocation);
            // No need to push to queue, corresponding memory will not be read
        } else {
            panic!("Tried to remove chunk that was not allocated. Chunk pos: {:?}", chunk_pos);
        }
    }

    pub fn reinsert(
        &mut self, 
        chunk_pos: IVec2, 
        data: &[U],
    ) {
        self.remove(chunk_pos);
        self.insert(chunk_pos, data);
    }

    pub fn get_buffer(&self) -> Arc<CpuAccessibleBuffer<[U]>> {
        self.buffer.get_current_buffer()
    }

    fn get_ind_commands(&self, multiplier: u32) -> Vec<DrawIndirectCommand> {
        self.allocations.values().map(|alloc| {
            DrawIndirectCommand {
                vertex_count: (alloc.back - alloc.front) * multiplier,
                instance_count: 1,
                first_vertex: alloc.front * multiplier,
                first_instance: 0,
            }
        }).collect()
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

    fn _get_back(&self, front: &T) -> Option<&T> {
        self.front_back.get(front)
    }

    fn _get_front(&self, back: &T) -> Option<&T> {
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